use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use yoda_core::{create_label_from_pixels, delete_label, LabelError, LabelObject, Point};
use yoda_data::{DatasetRepository, FilterMode, RepositoryError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum InteractionMode {
    #[default]
    Edit,
    Draw,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AccessMode {
    #[default]
    Locked,
    Unlocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ViewTransform {
    pub zoom: f32,
    pub pan_x: f32,
    pub pan_y: f32,
}

impl Default for ViewTransform {
    fn default() -> Self {
        Self {
            zoom: 1.0,
            pan_x: 0.0,
            pan_y: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageDimensions {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StatusMetadata {
    pub object_count: usize,
    pub status_text: Option<String>,
    pub error_text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AppState {
    pub current_labels: Vec<LabelObject>,
    pub current_label_path: Option<PathBuf>,
    pub current_image_path: Option<PathBuf>,
    pub image_dimensions: Option<ImageDimensions>,
    pub class_map: HashMap<u32, String>,
    pub hidden_classes: BTreeSet<u32>,
    pub selected_object_index: Option<usize>,
    pub interaction_mode: InteractionMode,
    pub drawing_vertices: Vec<Point>,
    pub drawing_class_id: u32,
    pub show_bbox: bool,
    pub show_segmask: bool,
    pub show_class_id: bool,
    pub show_class_name: bool,
    pub access_mode: AccessMode,
    pub view: ViewTransform,
    pub status: StatusMetadata,
    /// Classes to filter images by in the tree. Empty = no filter active.
    pub filter_classes: BTreeSet<u32>,
    /// Whether ALL or ANY of the selected filter classes must be present.
    pub filter_mode: FilterMode,
    /// Pre-fetched class index: dataset-relative path → list of class IDs.
    pub class_index: HashMap<String, Vec<u32>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_labels: Vec::new(),
            current_label_path: None,
            current_image_path: None,
            image_dimensions: None,
            class_map: HashMap::new(),
            hidden_classes: BTreeSet::new(),
            selected_object_index: None,
            interaction_mode: InteractionMode::Edit,
            drawing_vertices: Vec::new(),
            filter_classes: BTreeSet::new(),
            filter_mode: FilterMode::default(),
            class_index: HashMap::new(),
            drawing_class_id: 0,
            show_bbox: false,
            show_segmask: true,
            show_class_id: false,
            show_class_name: false,
            access_mode: AccessMode::Locked,
            view: ViewTransform::default(),
            status: StatusMetadata {
                object_count: 0,
                status_text: Some(String::from("No image loaded")),
                error_text: None,
            },
        }
    }
}

impl AppState {
    pub fn is_locked(&self) -> bool {
        self.access_mode == AccessMode::Locked
    }

    pub fn can_edit(&self) -> bool {
        !self.is_locked()
    }

    pub fn visible_labels(&self) -> Vec<LabelObject> {
        self.current_labels
            .iter()
            .cloned()
            .map(|mut label| {
                label.visible = label.visible && !self.hidden_classes.contains(&label.class_id);
                label
            })
            .collect()
    }

    pub fn delete_enabled(&self) -> bool {
        self.selected_object_index.is_some() && self.can_edit()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedImage {
    pub image_path: PathBuf,
    pub label_path: PathBuf,
    pub image_dimensions: ImageDimensions,
    pub labels: Vec<LabelObject>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppEffect {
    PersistLabels {
        image_path: PathBuf,
        labels: Vec<LabelObject>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditOperation {
    EnterDrawMode,
    ChangeLabelClass,
    DeleteLabel,
    FinishDrawing,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppAction {
    ImageLoaded(LoadedImage),
    ImageCleared,
    ClassMapLoaded(HashMap<u32, String>),
    ToggleSegmask,
    ToggleBbox,
    ToggleClassId,
    ToggleClassName,
    SetAccessMode(AccessMode),
    SetInteractionMode(InteractionMode),
    SetClassVisibility { class_id: u32, visible: bool },
    SetObjectVisibility { label_index: usize, visible: bool },
    ToggleSelection { label_index: Option<usize> },
    SelectLabel { label_index: Option<usize> },
    SetDrawClassId(u32),
    AddDrawingVertex(Point),
    CancelDrawing,
    FinishDrawing,
    ChangeLabelClass { label_index: usize, class_id: u32 },
    DeleteSelectedLabel,
    DeleteLabel { label_index: usize },
    SetZoom(f32),
    ZoomBy(f32),
    SetPan { x: f32, y: f32 },
    PanBy { dx: f32, dy: f32 },
    ResetView,
    ClassIndexLoaded(HashMap<String, Vec<u32>>),
    SetFilterClass { class_id: u32, selected: bool },
    ClearClassFilter,
    SetFilterMode(FilterMode),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActionResult {
    pub effects: Vec<AppEffect>,
    pub blocked: Option<EditOperation>,
}

impl ActionResult {
    const fn applied() -> Self {
        Self {
            effects: Vec::new(),
            blocked: None,
        }
    }

    fn with_effect(effect: AppEffect) -> Self {
        Self {
            effects: vec![effect],
            blocked: None,
        }
    }

    const fn blocked(operation: EditOperation) -> Self {
        Self {
            effects: Vec::new(),
            blocked: Some(operation),
        }
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Repository(#[from] RepositoryError),
    #[error(transparent)]
    Label(#[from] LabelError),
    #[error("no image loaded")]
    NoImageLoaded,
}

pub trait AppServices {
    fn load_image(&self, image_path: &Path) -> Result<LoadedImage, AppError>;
    fn persist_labels(&self, image_path: &Path, labels: &[LabelObject]) -> Result<(), AppError>;
    fn load_class_map(&self) -> Result<HashMap<u32, String>, AppError>;
}

#[derive(Debug, Clone)]
pub struct RepositoryBackedAppServices<R> {
    repository: R,
}

impl<R> RepositoryBackedAppServices<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub fn repository(&self) -> &R {
        &self.repository
    }
}

impl<R> AppServices for RepositoryBackedAppServices<R>
where
    R: DatasetRepository,
{
    fn load_image(&self, image_path: &Path) -> Result<LoadedImage, AppError> {
        let label_path = self.repository.label_path_for_image(image_path)?;
        let (width, height) = self.repository.image_dimensions(image_path)?;
        let labels = self.repository.load_labels(image_path)?;

        Ok(LoadedImage {
            image_path: image_path.to_path_buf(),
            label_path,
            image_dimensions: ImageDimensions { width, height },
            labels,
        })
    }

    fn persist_labels(&self, image_path: &Path, labels: &[LabelObject]) -> Result<(), AppError> {
        self.repository.save_labels(image_path, labels)?;
        Ok(())
    }

    fn load_class_map(&self) -> Result<HashMap<u32, String>, AppError> {
        Ok(self.repository.class_map()?)
    }
}

pub fn apply_action(state: &mut AppState, action: AppAction) -> Result<ActionResult, AppError> {
    let result = match action {
        AppAction::ImageLoaded(loaded) => {
            state.current_image_path = Some(loaded.image_path);
            state.current_label_path = Some(loaded.label_path);
            state.image_dimensions = Some(loaded.image_dimensions);
            state.current_labels = loaded.labels;
            state.selected_object_index = None;
            state.interaction_mode = InteractionMode::Edit;
            state.drawing_vertices.clear();
            state.view = ViewTransform::default();
            state.status.status_text = Some(String::from("Image loaded"));
            state.status.error_text = None;
            sync_status(state);
            ActionResult::applied()
        }
        AppAction::ImageCleared => {
            state.current_labels.clear();
            state.current_label_path = None;
            state.current_image_path = None;
            state.image_dimensions = None;
            state.selected_object_index = None;
            state.drawing_vertices.clear();
            state.interaction_mode = InteractionMode::Edit;
            state.view = ViewTransform::default();
            state.status.status_text = Some(String::from("No image loaded"));
            state.status.error_text = None;
            sync_status(state);
            ActionResult::applied()
        }
        AppAction::ClassMapLoaded(class_map) => {
            state.class_map = class_map;
            if state.drawing_class_id == 0 && !state.class_map.is_empty() {
                state.drawing_class_id = *state.class_map.keys().min().unwrap_or(&0);
            }
            ActionResult::applied()
        }
        AppAction::ToggleSegmask => {
            state.show_segmask = !state.show_segmask;
            ActionResult::applied()
        }
        AppAction::ToggleBbox => {
            state.show_bbox = !state.show_bbox;
            ActionResult::applied()
        }
        AppAction::ToggleClassId => {
            state.show_class_id = !state.show_class_id;
            ActionResult::applied()
        }
        AppAction::ToggleClassName => {
            state.show_class_name = !state.show_class_name;
            ActionResult::applied()
        }
        AppAction::SetAccessMode(access_mode) => {
            state.access_mode = access_mode;
            if state.is_locked() && state.interaction_mode == InteractionMode::Draw {
                state.interaction_mode = InteractionMode::Edit;
                state.drawing_vertices.clear();
            }
            state.status.error_text = None;
            ActionResult::applied()
        }
        AppAction::SetInteractionMode(mode) => {
            if mode == InteractionMode::Draw && state.is_locked() {
                state.status.error_text = Some(String::from("Unlock editing to draw."));
                ActionResult::blocked(EditOperation::EnterDrawMode)
            } else {
                state.interaction_mode = mode;
                if mode == InteractionMode::Edit {
                    state.drawing_vertices.clear();
                }
                state.status.error_text = None;
                ActionResult::applied()
            }
        }
        AppAction::SetClassVisibility { class_id, visible } => {
            if visible {
                state.hidden_classes.remove(&class_id);
            } else {
                state.hidden_classes.insert(class_id);
            }
            ActionResult::applied()
        }
        AppAction::SetObjectVisibility {
            label_index,
            visible,
        } => {
            if let Some(label) = state
                .current_labels
                .iter_mut()
                .find(|label| label.index == label_index)
            {
                label.visible = visible;
            }
            ActionResult::applied()
        }
        AppAction::ToggleSelection { label_index } => {
            state.selected_object_index = match (state.selected_object_index, label_index) {
                (Some(current), Some(next)) if current == next => None,
                (_, next) => next,
            };
            ActionResult::applied()
        }
        AppAction::SelectLabel { label_index } => {
            state.selected_object_index = label_index;
            ActionResult::applied()
        }
        AppAction::SetDrawClassId(class_id) => {
            state.drawing_class_id = class_id;
            ActionResult::applied()
        }
        AppAction::AddDrawingVertex(point) => {
            if state.interaction_mode == InteractionMode::Draw {
                state.drawing_vertices.push(point);
            }
            ActionResult::applied()
        }
        AppAction::CancelDrawing => {
            state.drawing_vertices.clear();
            state.interaction_mode = InteractionMode::Edit;
            ActionResult::applied()
        }
        AppAction::FinishDrawing => finish_drawing(state)?,
        AppAction::ChangeLabelClass {
            label_index,
            class_id,
        } => change_label_class(state, label_index, class_id),
        AppAction::DeleteSelectedLabel => {
            if let Some(label_index) = state.selected_object_index {
                delete_label_by_index(state, label_index)
            } else {
                ActionResult::applied()
            }
        }
        AppAction::DeleteLabel { label_index } => delete_label_by_index(state, label_index),
        AppAction::SetZoom(zoom) => {
            state.view.zoom = zoom.max(0.01);
            ActionResult::applied()
        }
        AppAction::ZoomBy(factor) => {
            state.view.zoom = (state.view.zoom * factor).clamp(0.01, 50.0);
            ActionResult::applied()
        }
        AppAction::SetPan { x, y } => {
            state.view.pan_x = x;
            state.view.pan_y = y;
            ActionResult::applied()
        }
        AppAction::PanBy { dx, dy } => {
            state.view.pan_x += dx;
            state.view.pan_y += dy;
            ActionResult::applied()
        }
        AppAction::ResetView => {
            state.view = ViewTransform::default();
            ActionResult::applied()
        }
        AppAction::ClassIndexLoaded(entries) => {
            state.class_index = entries;
            ActionResult::applied()
        }
        AppAction::SetFilterClass { class_id, selected } => {
            if selected {
                state.filter_classes.insert(class_id);
            } else {
                state.filter_classes.remove(&class_id);
            }
            ActionResult::applied()
        }
        AppAction::ClearClassFilter => {
            state.filter_classes.clear();
            ActionResult::applied()
        }
        AppAction::SetFilterMode(mode) => {
            state.filter_mode = mode;
            ActionResult::applied()
        }
    };

    sync_status(state);
    Ok(result)
}

fn finish_drawing(state: &mut AppState) -> Result<ActionResult, AppError> {
    if state.is_locked() {
        state.status.error_text = Some(String::from("Unlock editing to add objects."));
        return Ok(ActionResult::blocked(EditOperation::FinishDrawing));
    }

    let Some(image_dimensions) = state.image_dimensions else {
        return Err(AppError::NoImageLoaded);
    };
    if state.drawing_vertices.len() < 3 {
        return Ok(ActionResult::applied());
    }

    let points = state
        .drawing_vertices
        .iter()
        .map(|point| (point.x, point.y))
        .collect::<Vec<_>>();
    let new_label = create_label_from_pixels(
        &points,
        image_dimensions.width,
        image_dimensions.height,
        state.drawing_class_id,
        state.current_labels.len(),
    )?;
    state.current_labels.push(new_label);
    state.drawing_vertices.clear();
    state.interaction_mode = InteractionMode::Edit;
    state.status.error_text = None;

    Ok(save_effect_for_current_image(state))
}

fn change_label_class(state: &mut AppState, label_index: usize, class_id: u32) -> ActionResult {
    if state.is_locked() {
        state.status.error_text = Some(String::from("Unlock editing to change classes."));
        return ActionResult::blocked(EditOperation::ChangeLabelClass);
    }

    if let Some(label) = state
        .current_labels
        .iter_mut()
        .find(|label| label.index == label_index)
    {
        label.class_id = class_id;
    }
    state.status.error_text = None;
    save_effect_for_current_image(state)
}

fn delete_label_by_index(state: &mut AppState, label_index: usize) -> ActionResult {
    if state.is_locked() {
        state.status.error_text = Some(String::from("Unlock editing to delete objects."));
        return ActionResult::blocked(EditOperation::DeleteLabel);
    }

    state.current_labels = delete_label(&state.current_labels, label_index);
    if state.selected_object_index == Some(label_index) {
        state.selected_object_index = None;
    } else if let Some(selected_index) = state.selected_object_index {
        let valid_indices = state
            .current_labels
            .iter()
            .map(|label| label.index)
            .collect::<BTreeSet<_>>();
        if !valid_indices.contains(&selected_index) {
            state.selected_object_index = None;
        }
    }
    state.status.error_text = None;
    save_effect_for_current_image(state)
}

fn save_effect_for_current_image(state: &AppState) -> ActionResult {
    match &state.current_image_path {
        Some(image_path) => ActionResult::with_effect(AppEffect::PersistLabels {
            image_path: image_path.clone(),
            labels: state.current_labels.clone(),
        }),
        None => ActionResult::applied(),
    }
}

fn sync_status(state: &mut AppState) {
    state.status.object_count = state.current_labels.len();
    if state.status.status_text.is_none() {
        state.status.status_text = Some(if state.current_image_path.is_some() {
            String::from("Image loaded")
        } else {
            String::from("No image loaded")
        });
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::TempDir;
    use yoda_config::YoDaSettings;
    use yoda_data::LocalDatasetRepository;

    use super::{
        apply_action, AccessMode, AppAction, AppEffect, AppServices, AppState,
        EditOperation, ImageDimensions, InteractionMode, LoadedImage, Point,
        RepositoryBackedAppServices,
    };

    fn sample_dataset() -> TempDir {
        let temp = TempDir::new().expect("create temp dir");
        let image_dir = temp.path().join("images/train");
        let label_dir = temp.path().join("labels/train");
        fs::create_dir_all(&image_dir).expect("create image dir");
        fs::create_dir_all(&label_dir).expect("create label dir");

        let image = image::RgbImage::from_pixel(640, 480, image::Rgb([128, 128, 128]));
        image.save(image_dir.join("test1.jpg")).expect("save jpg");
        image.save(image_dir.join("test2.png")).expect("save png");
        fs::write(
            label_dir.join("test1.txt"),
            "0 0.1 0.2 0.3 0.2 0.3 0.8 0.1 0.8 0.05 0.5\n2 0.5 0.5 0.7 0.5 0.7 0.9 0.5 0.9\n",
        )
        .expect("write labels");
        fs::write(label_dir.join("test2.txt"), "1 0.5 0.5 0.4 0.6\n").expect("write labels");

        temp
    }

    fn load_state_for_test() -> (AppState, LoadedImage) {
        let temp = sample_dataset();
        let settings = YoDaSettings {
            image_base_path: temp.path().join("images"),
            label_base_path: temp.path().join("labels"),
            class_info: None,
            color_map: None,
            host: None,
            port: 8080,
        };
        let repository = LocalDatasetRepository::new(settings);
        let services = RepositoryBackedAppServices::new(repository);
        let loaded = services
            .load_image(&temp.path().join("images/train/test1.jpg"))
            .expect("load image");
        (AppState::default(), loaded)
    }

    #[test]
    fn defaults_to_locked_viewer_state() {
        let state = AppState::default();
        assert_eq!(state.access_mode, AccessMode::Locked);
        assert_eq!(state.interaction_mode, InteractionMode::Edit);
        assert!(state.show_segmask);
        assert!(!state.show_bbox);
        assert_eq!(state.status.object_count, 0);
    }

    #[test]
    fn image_load_preserves_lock_state_across_images() {
        let temp = sample_dataset();
        let settings = YoDaSettings {
            image_base_path: temp.path().join("images"),
            label_base_path: temp.path().join("labels"),
            class_info: None,
            color_map: None,
            host: None,
            port: 8080,
        };
        let services = RepositoryBackedAppServices::new(LocalDatasetRepository::new(settings));
        let mut state = AppState::default();
        state.access_mode = AccessMode::Unlocked;

        let first = services
            .load_image(&temp.path().join("images/train/test1.jpg"))
            .expect("load first image");
        apply_action(&mut state, AppAction::ImageLoaded(first)).expect("apply first load");
        let second = services
            .load_image(&temp.path().join("images/train/test2.png"))
            .expect("load second image");
        apply_action(&mut state, AppAction::ImageLoaded(second)).expect("apply second load");

        assert_eq!(state.access_mode, AccessMode::Unlocked);
        assert_eq!(state.selected_object_index, None);
        assert_eq!(state.current_labels.len(), 1);
    }

    #[test]
    fn locked_state_blocks_edit_actions() {
        let (mut state, loaded) = load_state_for_test();
        apply_action(&mut state, AppAction::ImageLoaded(loaded)).expect("load state");
        state.selected_object_index = Some(0);
        state.interaction_mode = InteractionMode::Draw;
        state.drawing_vertices = vec![Point::new(10.0, 10.0), Point::new(20.0, 10.0), Point::new(20.0, 20.0)];

        let draw_mode = apply_action(&mut state, AppAction::SetInteractionMode(InteractionMode::Draw))
            .expect("apply draw mode");
        assert_eq!(draw_mode.blocked, Some(EditOperation::EnterDrawMode));

        let delete = apply_action(&mut state, AppAction::DeleteSelectedLabel).expect("delete");
        assert_eq!(delete.blocked, Some(EditOperation::DeleteLabel));

        let change = apply_action(
            &mut state,
            AppAction::ChangeLabelClass {
                label_index: 0,
                class_id: 9,
            },
        )
        .expect("change class");
        assert_eq!(change.blocked, Some(EditOperation::ChangeLabelClass));

        let finish = apply_action(&mut state, AppAction::FinishDrawing).expect("finish drawing");
        assert_eq!(finish.blocked, Some(EditOperation::FinishDrawing));
    }

    #[test]
    fn class_and_object_visibility_both_apply() {
        let (mut state, loaded) = load_state_for_test();
        apply_action(&mut state, AppAction::ImageLoaded(loaded)).expect("load state");

        apply_action(
            &mut state,
            AppAction::SetObjectVisibility {
                label_index: 0,
                visible: false,
            },
        )
        .expect("set object visibility");
        apply_action(
            &mut state,
            AppAction::SetClassVisibility {
                class_id: 2,
                visible: false,
            },
        )
        .expect("set class visibility");

        let visible = state.visible_labels();
        assert!(!visible[0].visible);
        assert!(!visible[1].visible);
    }

    #[test]
    fn delete_clears_invalid_selection() {
        let (mut state, loaded) = load_state_for_test();
        state.access_mode = AccessMode::Unlocked;
        apply_action(&mut state, AppAction::ImageLoaded(loaded)).expect("load state");
        state.selected_object_index = Some(1);

        let result = apply_action(&mut state, AppAction::DeleteLabel { label_index: 1 })
            .expect("delete label");
        assert_eq!(state.selected_object_index, None);
        assert_eq!(state.current_labels.len(), 1);
        assert!(matches!(result.effects[0], AppEffect::PersistLabels { .. }));
    }

    #[test]
    fn drawing_lifecycle_appends_label_and_requests_save() {
        let (mut state, loaded) = load_state_for_test();
        state.access_mode = AccessMode::Unlocked;
        apply_action(&mut state, AppAction::ImageLoaded(loaded)).expect("load state");
        apply_action(&mut state, AppAction::SetDrawClassId(7)).expect("set draw class");
        apply_action(&mut state, AppAction::SetInteractionMode(InteractionMode::Draw))
            .expect("set draw mode");
        apply_action(&mut state, AppAction::AddDrawingVertex(Point::new(10.0, 10.0)))
            .expect("add vertex 1");
        apply_action(&mut state, AppAction::AddDrawingVertex(Point::new(20.0, 10.0)))
            .expect("add vertex 2");
        apply_action(&mut state, AppAction::AddDrawingVertex(Point::new(20.0, 20.0)))
            .expect("add vertex 3");

        let result = apply_action(&mut state, AppAction::FinishDrawing).expect("finish drawing");
        assert_eq!(state.current_labels.len(), 3);
        assert_eq!(state.current_labels[2].class_id, 7);
        assert!(state.drawing_vertices.is_empty());
        assert_eq!(state.interaction_mode, InteractionMode::Edit);
        assert!(matches!(result.effects[0], AppEffect::PersistLabels { .. }));
    }

    #[test]
    fn service_loads_repository_backed_image_state() {
        let temp = sample_dataset();
        let settings = YoDaSettings {
            image_base_path: temp.path().join("images"),
            label_base_path: temp.path().join("labels"),
            class_info: None,
            color_map: None,
            host: None,
            port: 8080,
        };
        let services = RepositoryBackedAppServices::new(LocalDatasetRepository::new(settings));

        let loaded = services
            .load_image(Path::new(&temp.path().join("images/train/test2.png")))
            .expect("load image");
        assert_eq!(loaded.image_dimensions, ImageDimensions { width: 640, height: 480 });
        assert_eq!(loaded.labels.len(), 1);
        assert!(loaded.label_path.ends_with("test2.txt"));
    }

    #[test]
    fn service_persists_labels_through_repository() {
        let temp = sample_dataset();
        let settings = YoDaSettings {
            image_base_path: temp.path().join("images"),
            label_base_path: temp.path().join("labels"),
            class_info: None,
            color_map: None,
            host: None,
            port: 8080,
        };
        let services = RepositoryBackedAppServices::new(LocalDatasetRepository::new(settings));
        let mut loaded = services
            .load_image(Path::new(&temp.path().join("images/train/test2.png")))
            .expect("load image");
        loaded.labels[0].class_id = 8;

        services
            .persist_labels(&loaded.image_path, &loaded.labels)
            .expect("persist labels");

        let written = fs::read_to_string(temp.path().join("labels/train/test2.txt"))
            .expect("read labels");
        assert!(written.starts_with("8 "));
    }
}