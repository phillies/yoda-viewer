#![cfg(feature = "server")]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::extract::{Extension, Query};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use dioxus_server::{DioxusRouterExt, FullstackState, ServeConfig};
use serde::{Deserialize, Serialize};
use tower_http::trace::TraceLayer;
use yoda_app::AppServices;
use yoda_app::RepositoryBackedAppServices;
use yoda_config::YoDaSettings;
use yoda_core::{render_labels_to_svg, LabelObject, RenderOptions};
use yoda_data::{DatasetRepository, LocalDatasetRepository, TreeNode};
use yoda_ui::{RootApp, PAN_ZOOM_SCRIPT};

const FALLBACK_CSS: &str = r#"
:root {
        color-scheme: dark;
        --bg: #11161c;
        --panel: #19222b;
        --panel-2: #202c38;
        --line: #344557;
        --text: #ecf3f8;
        --muted: #94a6b8;
        --accent: #e25d3f;
        --accent-soft: rgba(226, 93, 63, 0.15);
}
* { box-sizing: border-box; }
html, body { margin: 0; min-height: 100%; background: radial-gradient(circle at top, #1a2430, #0d1217 58%); color: var(--text); font-family: "Segoe UI", sans-serif; }
body { overflow: hidden; }
a { color: inherit; text-decoration: none; }
.shell { display: grid; grid-template-columns: 320px minmax(0, 1fr) 320px; height: 100vh; }
.panel { background: rgba(25, 34, 43, 0.92); border-right: 1px solid var(--line); min-height: 0; overflow: auto; }
.panel.right { border-right: none; border-left: 1px solid var(--line); }
.section-title { padding: 16px 18px 10px; font-size: 11px; letter-spacing: 0.18em; text-transform: uppercase; color: var(--muted); }
.tree { padding: 0 12px 16px; }
.tree details { margin-bottom: 6px; }
.tree summary { cursor: pointer; list-style: none; display: flex; align-items: center; gap: 8px; padding: 8px 10px; border-radius: 10px; }
.tree summary::-webkit-details-marker { display: none; }
.tree a, .tree .folder { display: flex; align-items: center; gap: 8px; padding: 8px 10px; border-radius: 10px; color: inherit; }
.tree a:hover, .tree summary:hover { background: rgba(255,255,255,0.04); }
.tree a.selected { background: var(--accent-soft); border: 1px solid rgba(226, 93, 63, 0.35); }
.tree-children { margin-left: 18px; }
.icon { width: 16px; height: 16px; display: inline-flex; align-items: center; justify-content: center; color: var(--muted); flex: none; }
.content { display: flex; flex-direction: column; min-width: 0; min-height: 0; }
.toolbar { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; padding: 16px 18px; border-bottom: 1px solid var(--line); background: rgba(17, 22, 28, 0.72); }
.pill { padding: 8px 12px; border-radius: 999px; border: 1px solid var(--line); background: rgba(255,255,255,0.03); font-size: 12px; }
.message { margin: 14px 18px 0; padding: 10px 12px; border-radius: 12px; font-size: 13px; }
.message.warn { background: rgba(255, 180, 84, 0.12); border: 1px solid rgba(255, 180, 84, 0.25); color: #ffd08a; }
.message.error { background: rgba(255, 88, 88, 0.12); border: 1px solid rgba(255, 88, 88, 0.2); color: #ffb0b0; }
.viewport-wrap { flex: 1; overflow: auto; padding: 18px; }
.viewport { min-height: 100%; border: 1px solid var(--line); border-radius: 22px; background: linear-gradient(180deg, rgba(255,255,255,0.02), rgba(255,255,255,0.01)); display: flex; align-items: center; justify-content: center; padding: 18px; overflow: hidden; }
.canvas-stage { transform-origin: top left; will-change: transform; }
.canvas { position: relative; display: inline-block; line-height: 0; }
.canvas img.main-image { display: block; max-width: min(100%, 1100px); max-height: calc(100vh - 220px); border-radius: 18px; box-shadow: 0 30px 80px rgba(0,0,0,0.45); }
.canvas img.overlay-image { position: absolute; inset: 0; width: 100%; height: 100%; pointer-events: none; }
.status { display: flex; gap: 12px; flex-wrap: wrap; padding: 12px 16px; border-top: 1px solid var(--line); color: var(--muted); font-size: 12px; background: rgba(17, 22, 28, 0.75); }
.empty { max-width: 420px; text-align: center; color: var(--muted); }
.legend, .objects { padding: 0 12px 18px; }
.legend-item, .object-item { display: flex; align-items: center; gap: 10px; padding: 10px 12px; margin-bottom: 8px; border-radius: 14px; background: rgba(255,255,255,0.03); }
.swatch { width: 12px; height: 12px; border-radius: 999px; flex: none; }
.grow { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.meta { color: var(--muted); font-size: 12px; }
@media (max-width: 1100px) {
    .shell { grid-template-columns: 260px minmax(0, 1fr); grid-template-rows: minmax(0, 1fr) 280px; }
    .panel.right { grid-column: 1 / span 2; border-left: none; border-top: 1px solid var(--line); }
}
@media (max-width: 860px) {
    body { overflow: auto; }
    .shell { display: block; height: auto; }
    .panel, .panel.right { border: none; border-bottom: 1px solid var(--line); }
}
"#;

#[derive(Debug, Clone)]
pub struct BackendState {
    repository: LocalDatasetRepository,
    image_root: PathBuf,
}

impl BackendState {
    pub fn from_settings(mut settings: YoDaSettings) -> Result<Self, ApiError> {
        let image_root = canonical_dir(&settings.image_base_path)?;
        settings.image_base_path = image_root.clone();
        if settings.label_base_path.exists() {
            settings.label_base_path = canonical_dir(&settings.label_base_path)?;
        }

        Ok(Self {
            repository: LocalDatasetRepository::new(settings),
            image_root,
        })
    }

    fn services(&self) -> RepositoryBackedAppServices<LocalDatasetRepository> {
        RepositoryBackedAppServices::new(self.repository.clone())
    }
}

#[derive(Debug, Clone, Deserialize)]
struct PathQuery {
    path: String,
}

#[derive(Debug, Clone, Deserialize)]
struct ImagePathQuery {
    image_path: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct ViewerQuery {
    image_path: Option<String>,
}

#[derive(Debug, Clone)]
struct FallbackViewerState {
    tree_html: String,
    image_src: Option<String>,
    overlay_src: Option<String>,
    image_name: String,
    dimensions_text: String,
    object_count: usize,
    class_legend_html: String,
    object_list_html: String,
    status_message: Option<String>,
    error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveLabelsRequest {
    pub labels: Vec<LabelObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeNodesResponse {
    pub nodes: Vec<TreeNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMetadataResponse {
    pub image_path: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabelsResponse {
    pub image_path: String,
    pub label_path: String,
    pub width: u32,
    pub height: u32,
    pub labels: Vec<LabelObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassMapResponse {
    pub class_map: std::collections::HashMap<u32, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorMapResponse {
    pub color_map: std::collections::HashMap<u32, [u8; 3]>,
}

#[derive(Debug, Clone, Serialize)]
struct ErrorBody {
    code: &'static str,
    message: String,
}

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    code: &'static str,
    message: String,
}

impl ApiError {
    fn new(status: StatusCode, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }

    fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, "bad_request", message)
    }

    fn forbidden(message: impl Into<String>) -> Self {
        Self::new(StatusCode::FORBIDDEN, "forbidden", message)
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", message)
    }

    fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "internal_error", message)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(ErrorBody {
                code: self.code,
                message: self.message,
            }),
        )
            .into_response()
    }
}

impl From<std::io::Error> for ApiError {
    fn from(error: std::io::Error) -> Self {
        ApiError::internal(error.to_string())
    }
}

impl From<yoda_app::AppError> for ApiError {
    fn from(error: yoda_app::AppError) -> Self {
        ApiError::internal(error.to_string())
    }
}

impl From<yoda_data::RepositoryError> for ApiError {
    fn from(error: yoda_data::RepositoryError) -> Self {
        ApiError::internal(error.to_string())
    }
}

pub fn build_router(settings: YoDaSettings) -> Result<Router, ApiError> {
    let public_dir = ensure_public_dir()?;
    let state = Arc::new(BackendState::from_settings(settings)?);
    let has_client_assets = log_public_assets_state(&public_dir);

    if has_client_assets {
        Ok(Router::<FullstackState>::new()
            .nest("/api", api_router::<FullstackState>(state.clone()))
            .layer(TraceLayer::new_for_http())
            .serve_dioxus_application(ServeConfig::new(), RootApp))
    } else {
        Ok(Router::new()
            .nest("/api", api_router::<()>(state.clone()))
            .route("/", get(render_fallback_viewer))
            .route("/favicon.ico", get(favicon))
            .layer(Extension(state))
            .layer(TraceLayer::new_for_http()))
    }
}

pub fn build_api_router(settings: YoDaSettings) -> Result<Router, ApiError> {
    let state = Arc::new(BackendState::from_settings(settings)?);
    Ok(Router::<FullstackState>::new()
        .nest("/api", api_router::<FullstackState>(state))
        .serve_api_application(ServeConfig::new(), RootApp))
}

fn api_router<S>(state: Arc<BackendState>) -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/health", get(health))
        .route("/tree", get(list_tree))
        .route("/tree/children", get(list_children))
        .route("/image", get(image_bytes))
        .route("/image/metadata", get(image_metadata))
        .route("/labels", get(load_labels).put(save_labels))
        .route("/class-map", get(class_map))
        .route("/color-map", get(color_map))
        .layer(Extension(state))
}

fn ensure_public_dir() -> Result<PathBuf, ApiError> {
    let executable = std::env::current_exe()?;
    let Some(parent) = executable.parent() else {
        return Err(ApiError::internal("unable to determine executable directory"));
    };

    let public_dir = parent.join("public");
    fs::create_dir_all(&public_dir)?;
    Ok(public_dir)
}

fn log_public_assets_state(public_dir: &Path) -> bool {
    let has_client_assets = public_dir.join("index.html").is_file() || fs::read_dir(public_dir)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .any(|entry| {
            let path = entry.path();
            if path.is_dir() {
                return fs::read_dir(&path)
                    .ok()
                    .into_iter()
                    .flatten()
                    .flatten()
                    .any(|nested| {
                        nested
                            .path()
                            .extension()
                            .and_then(|extension| extension.to_str())
                            .is_some_and(|extension| matches!(extension, "js" | "mjs" | "wasm"))
                    });
            }

            path.extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| matches!(extension, "js" | "mjs" | "wasm"))
        });

    if has_client_assets {
        tracing::info!(public_dir = %public_dir.display(), "detected Dioxus client assets");
    } else {
        tracing::warn!(
            public_dir = %public_dir.display(),
            "no Dioxus client assets found; the browser page will render server-side HTML but will not hydrate without a built web bundle"
        );
    }

    has_client_assets
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: String::from("ok"),
        version: String::from(env!("CARGO_PKG_VERSION")),
    })
}

async fn list_tree(Extension(state): Extension<Arc<BackendState>>) -> Result<Json<TreeNodesResponse>, ApiError> {
    tracing::info!(image_root = %state.image_root.display(), "listing dataset root");
    Ok(Json(TreeNodesResponse {
        nodes: state.repository.list_root_nodes()?,
    }))
}

async fn list_children(
    Extension(state): Extension<Arc<BackendState>>,
    Query(query): Query<PathQuery>,
) -> Result<Json<TreeNodesResponse>, ApiError> {
    let path = resolve_path(&state.image_root, &query.path)?;
    if !path.is_dir() {
        return Err(ApiError::not_found(format!("directory not found: {}", path.display())));
    }

    tracing::info!(path = %path.display(), "expanding dataset directory");

    Ok(Json(TreeNodesResponse {
        nodes: state.repository.expand_directory(&path)?,
    }))
}

async fn image_metadata(
    Extension(state): Extension<Arc<BackendState>>,
    Query(query): Query<ImagePathQuery>,
) -> Result<Json<ImageMetadataResponse>, ApiError> {
    let image_path = resolve_path(&state.image_root, &query.image_path)?;
    ensure_file(&image_path)?;
    let (width, height) = state.repository.image_dimensions(&image_path)?;

    Ok(Json(ImageMetadataResponse {
        image_path: image_path.to_string_lossy().into_owned(),
        width,
        height,
    }))
}

async fn image_bytes(
    Extension(state): Extension<Arc<BackendState>>,
    Query(query): Query<ImagePathQuery>,
) -> Result<Response, ApiError> {
    let image_path = resolve_path(&state.image_root, &query.image_path)?;
    ensure_file(&image_path)?;
    tracing::info!(image_path = %image_path.display(), "serving image bytes");
    let bytes = state.repository.image_bytes(&image_path)?;
    let content_type = mime_type_for_image(&image_path);

    Ok((
        [(header::CONTENT_TYPE, HeaderValue::from_static(content_type))],
        bytes,
    )
        .into_response())
}

async fn load_labels(
    Extension(state): Extension<Arc<BackendState>>,
    Query(query): Query<ImagePathQuery>,
) -> Result<Json<LabelsResponse>, ApiError> {
    let image_path = resolve_path(&state.image_root, &query.image_path)?;
    ensure_file(&image_path)?;
    tracing::info!(image_path = %image_path.display(), "loading image labels");
    let services = state.services();
    let loaded = services.load_image(&image_path)?;

    Ok(Json(LabelsResponse {
        image_path: loaded.image_path.to_string_lossy().into_owned(),
        label_path: loaded.label_path.to_string_lossy().into_owned(),
        width: loaded.image_dimensions.width,
        height: loaded.image_dimensions.height,
        labels: loaded.labels,
    }))
}

async fn save_labels(
    Extension(state): Extension<Arc<BackendState>>,
    Query(query): Query<ImagePathQuery>,
    Json(payload): Json<SaveLabelsRequest>,
) -> Result<Json<LabelsResponse>, ApiError> {
    let image_path = resolve_path(&state.image_root, &query.image_path)?;
    ensure_file(&image_path)?;
    tracing::info!(image_path = %image_path.display(), label_count = payload.labels.len(), "persisting image labels");
    let services = state.services();
    services.persist_labels(&image_path, &payload.labels)?;
    let loaded = services.load_image(&image_path)?;

    Ok(Json(LabelsResponse {
        image_path: loaded.image_path.to_string_lossy().into_owned(),
        label_path: loaded.label_path.to_string_lossy().into_owned(),
        width: loaded.image_dimensions.width,
        height: loaded.image_dimensions.height,
        labels: loaded.labels,
    }))
}

async fn class_map(Extension(state): Extension<Arc<BackendState>>) -> Result<Json<ClassMapResponse>, ApiError> {
    let services = state.services();
    Ok(Json(ClassMapResponse {
        class_map: services.load_class_map()?,
    }))
}

async fn color_map(Extension(state): Extension<Arc<BackendState>>) -> Result<Json<ColorMapResponse>, ApiError> {
    let tuples = state.repository.color_map()?;
    let color_map = tuples
        .into_iter()
        .map(|(class_id, (r, g, b))| (class_id, [r, g, b]))
        .collect();

    Ok(Json(ColorMapResponse { color_map }))
}

async fn render_fallback_viewer(
    Extension(state): Extension<Arc<BackendState>>,
    Query(query): Query<ViewerQuery>,
) -> Result<Response, ApiError> {
    let view = build_fallback_view(&state, query.image_path.as_deref())?;
    let html = render_fallback_html(&view);

    Ok((
        [(header::CONTENT_TYPE, HeaderValue::from_static("text/html; charset=utf-8"))],
        html,
    )
        .into_response())
}

async fn favicon() -> Response {
    StatusCode::NO_CONTENT.into_response()
}

fn build_fallback_view(
    state: &BackendState,
    requested_image: Option<&str>,
) -> Result<FallbackViewerState, ApiError> {
    let services = state.services();
    let class_map = services.load_class_map()?;
    let color_map = state.repository.color_map()?;
    let selected_image_path = match requested_image {
        Some(image_path) => Some(resolve_path(&state.image_root, image_path)?),
        None => find_first_image(&state.image_root)?,
    };

    let loaded = if let Some(image_path) = selected_image_path.as_deref() {
        tracing::info!(image_path = %image_path.display(), "rendering fallback web viewer image");
        Some(services.load_image(image_path)?)
    } else {
        None
    };

    let tree_html = render_tree_html(
        &state.repository,
        &state.image_root,
        loaded.as_ref().map(|loaded| loaded.image_path.as_path()),
    )?;

    let (image_src, overlay_src, image_name, dimensions_text, object_count, class_legend_html, object_list_html, status_message) =
        if let Some(loaded) = loaded {
            let image_path = dataset_relative_path(&state.image_root, &loaded.image_path);
            let image_src = format!(
                "/api/image?image_path={}",
                urlencoding::encode(&image_path)
            );
            let visible_labels = loaded.labels.clone();
            let overlay_svg = render_labels_to_svg(
                &visible_labels,
                Some(&color_map),
                Some(&class_map),
                &RenderOptions {
                    show_bbox: true,
                    show_segmask: true,
                    show_class_id: false,
                    show_class_name: true,
                    selected_index: None,
                },
            );
            let overlay_src = Some(build_overlay_data_uri(
                loaded.image_dimensions.width,
                loaded.image_dimensions.height,
                &overlay_svg,
            ));
            let class_legend_html = render_class_legend_html(&visible_labels, &class_map, &color_map);
            let object_list_html = render_object_list_html(&visible_labels, &class_map, &color_map);

            (
                Some(image_src),
                overlay_src,
                loaded
                    .image_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("Unknown")
                    .to_string(),
                format!("{} x {}", loaded.image_dimensions.width, loaded.image_dimensions.height),
                visible_labels.len(),
                class_legend_html,
                object_list_html,
                Some(String::from("Rendered from server-side fallback viewer")),
            )
        } else {
            (
                None,
                None,
                String::from("None"),
                String::from("-"),
                0,
                String::new(),
                String::new(),
                Some(String::from("No image found in dataset")),
            )
        };

    Ok(FallbackViewerState {
        tree_html,
        image_src,
        overlay_src,
        image_name,
        dimensions_text,
        object_count,
        class_legend_html,
        object_list_html,
        status_message,
        error_message: None,
    })
}

fn render_fallback_html(view: &FallbackViewerState) -> String {
    let image_html = match (&view.image_src, &view.overlay_src) {
        (Some(image_src), Some(overlay_src)) => format!(
            "<div class=\"canvas-stage\" data-panzoom-stage=\"true\" data-image-key=\"{}\"><div class=\"canvas\"><img class=\"main-image\" src=\"{}\" alt=\"Selected dataset image\" /><img class=\"overlay-image\" src=\"{}\" alt=\"Label overlay\" /></div></div>",
            escape_attr(image_src),
            escape_attr(image_src),
            escape_attr(overlay_src),
        ),
        (Some(image_src), None) => format!(
            "<div class=\"canvas-stage\" data-panzoom-stage=\"true\" data-image-key=\"{}\"><div class=\"canvas\"><img class=\"main-image\" src=\"{}\" alt=\"Selected dataset image\" /></div></div>",
            escape_attr(image_src),
            escape_attr(image_src),
        ),
        _ => String::from("<div class=\"empty\"><h2>Viewer Shell Ready</h2><p>Select an image from the dataset tree to render it on the server.</p></div>"),
    };

    let status_html = view
        .status_message
        .as_ref()
        .map(|message| format!("<div class=\"message warn\">{}</div>", escape_html(message)))
        .unwrap_or_default();
    let error_html = view
        .error_message
        .as_ref()
        .map(|message| format!("<div class=\"message error\">{}</div>", escape_html(message)))
        .unwrap_or_default();

    format!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\" /><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" /><title>YoDa Rust</title><style>{}</style><script>{}</script></head><body><main class=\"shell\"><aside class=\"panel\"><div class=\"section-title\">Dataset</div><div class=\"tree\">{}</div></aside><section class=\"content\"><div class=\"toolbar\"><span class=\"pill\">Server-rendered fallback</span><span class=\"pill\">Use dx serve for hydrated web UI</span></div>{}{}<div class=\"viewport-wrap\"><div class=\"viewport\" data-panzoom-container=\"true\">{}</div></div><div class=\"status\"><span>Image: {}</span><span>Dimensions: {}</span><span>Objects: {}</span><span>Mode: Viewer</span></div></section><aside class=\"panel right\"><div class=\"section-title\">Classes</div><div class=\"legend\">{}</div><div class=\"section-title\">Objects</div><div class=\"objects\">{}</div></aside></main></body></html>",
        FALLBACK_CSS,
        PAN_ZOOM_SCRIPT,
        view.tree_html,
        status_html,
        error_html,
        image_html,
        escape_html(&view.image_name),
        escape_html(&view.dimensions_text),
        view.object_count,
        view.class_legend_html,
        view.object_list_html,
    )
}

fn render_tree_html(
    repository: &LocalDatasetRepository,
    root: &Path,
    selected_image: Option<&Path>,
) -> Result<String, ApiError> {
    let mut html = String::new();
    for entry in sorted_entries(root)? {
        html.push_str(&render_tree_entry_html(repository, root, &entry, selected_image)?);
    }
    Ok(html)
}

fn render_tree_entry_html(
    repository: &LocalDatasetRepository,
    root: &Path,
    path: &Path,
    selected_image: Option<&Path>,
) -> Result<String, ApiError> {
    let label = escape_html(path.file_name().and_then(|name| name.to_str()).unwrap_or("?"));

    if path.is_dir() {
        let mut children_html = String::new();
        for child in sorted_entries(path)? {
            children_html.push_str(&render_tree_entry_html(repository, root, &child, selected_image)?);
        }

        return Ok(format!(
            "<details open><summary><span class=\"icon\">{}</span><span>{}</span></summary><div class=\"tree-children\">{}</div></details>",
            folder_icon_svg(),
            label,
            children_html,
        ));
    }

    if !is_image_path(path) {
        return Ok(String::new());
    }

    let image_path = dataset_relative_path(root, path);
    let href = format!(
        "/?image_path={}",
        urlencoding::encode(&image_path)
    );
    let class = if selected_image == Some(path) {
        "selected"
    } else {
        ""
    };

    let _ = repository;
    Ok(format!(
        "<a class=\"{}\" href=\"{}\"><span class=\"icon\">{}</span><span>{}</span></a>",
        class,
        escape_attr(&href),
        image_icon_svg(),
        label,
    ))
}

fn render_class_legend_html(
    labels: &[LabelObject],
    class_map: &std::collections::HashMap<u32, String>,
    color_map: &std::collections::HashMap<u32, (u8, u8, u8)>,
) -> String {
    let mut seen = std::collections::BTreeSet::new();
    let mut html = String::new();
    for label in labels {
        if !seen.insert(label.class_id) {
            continue;
        }
        let class_name = class_map
            .get(&label.class_id)
            .cloned()
            .unwrap_or_else(|| format!("class {}", label.class_id));
        let (r, g, b) = color_map
            .get(&label.class_id)
            .copied()
            .unwrap_or_else(|| yoda_core::default_color_for_class(label.class_id));
        html.push_str(&format!(
            "<div class=\"legend-item\"><span class=\"swatch\" style=\"background: rgb({},{},{});\"></span><span class=\"grow\">{}</span><span class=\"meta\">#{}</span></div>",
            r,
            g,
            b,
            escape_html(&class_name),
            label.class_id,
        ));
    }
    html
}

fn render_object_list_html(
    labels: &[LabelObject],
    class_map: &std::collections::HashMap<u32, String>,
    color_map: &std::collections::HashMap<u32, (u8, u8, u8)>,
) -> String {
    let mut html = String::new();
    for label in labels {
        let class_name = class_map
            .get(&label.class_id)
            .cloned()
            .unwrap_or_else(|| format!("class {}", label.class_id));
        let (r, g, b) = color_map
            .get(&label.class_id)
            .copied()
            .unwrap_or_else(|| yoda_core::default_color_for_class(label.class_id));
        let object_kind = match label.label_type {
            yoda_core::LabelType::Bbox => "bbox",
            yoda_core::LabelType::Polygon => "polygon",
        };
        html.push_str(&format!(
            "<div class=\"object-item\"><span class=\"swatch\" style=\"background: rgb({},{},{});\"></span><span class=\"grow\">#{}</span><span class=\"grow\">{}</span><span class=\"meta\">{}</span></div>",
            r,
            g,
            b,
            label.index + 1,
            escape_html(&class_name),
            object_kind,
        ));
    }
    html
}

fn build_overlay_data_uri(width: u32, height: u32, inner_svg: &str) -> String {
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" width=\"{}\" height=\"{}\">{}</svg>",
        width,
        height,
        width,
        height,
        inner_svg,
    );
    format!("data:image/svg+xml;utf8,{}", urlencoding::encode(&svg))
}

fn sorted_entries(path: &Path) -> Result<Vec<PathBuf>, ApiError> {
    let entries = fs::read_dir(path)?;
    let mut items = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| !name.starts_with('.'))
                .unwrap_or(false)
        })
        .filter(|path| path.is_dir() || is_image_path(path))
        .collect::<Vec<_>>();

    items.sort_by(|left, right| {
        let left_key = (!left.is_dir(), lower_file_name(left));
        let right_key = (!right.is_dir(), lower_file_name(right));
        left_key.cmp(&right_key)
    });
    Ok(items)
}

fn find_first_image(root: &Path) -> Result<Option<PathBuf>, ApiError> {
    for entry in sorted_entries(root)? {
        if entry.is_dir() {
            if let Some(found) = find_first_image(&entry)? {
                return Ok(Some(found));
            }
        } else {
            return Ok(Some(entry));
        }
    }
    Ok(None)
}

fn is_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| matches!(extension.to_ascii_lowercase().as_str(), "jpg" | "jpeg" | "png" | "bmp" | "webp"))
}

fn lower_file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

fn dataset_relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn folder_icon_svg() -> &'static str {
    "<svg width=\"16\" height=\"16\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><path d=\"M3 7.5a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z\" /></svg>"
}

fn image_icon_svg() -> &'static str {
    "<svg width=\"16\" height=\"16\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"><rect x=\"4\" y=\"5\" width=\"16\" height=\"14\" rx=\"2\" /><circle cx=\"9\" cy=\"10\" r=\"1.5\" /><path d=\"m20 16-4.5-4.5L8 19\" /></svg>"
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn escape_attr(value: &str) -> String {
    escape_html(value)
}

fn resolve_path(root: &Path, raw_path: &str) -> Result<PathBuf, ApiError> {
    let candidate = if Path::new(raw_path).is_absolute() {
        PathBuf::from(raw_path)
    } else {
        root.join(raw_path)
    };

    let canonical = fs::canonicalize(&candidate).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            ApiError::not_found(format!("path not found: {}", candidate.display()))
        } else {
            ApiError::internal(error.to_string())
        }
    })?;

    if canonical.strip_prefix(root).is_err() {
        return Err(ApiError::forbidden(format!(
            "path is outside dataset root: {}",
            candidate.display()
        )));
    }

    Ok(canonical)
}

fn ensure_file(path: &Path) -> Result<(), ApiError> {
    if !path.is_file() {
        return Err(ApiError::not_found(format!("file not found: {}", path.display())));
    }

    Ok(())
}

fn canonical_dir(path: &Path) -> Result<PathBuf, ApiError> {
    let canonical = fs::canonicalize(path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            ApiError::not_found(format!("directory not found: {}", path.display()))
        } else {
            ApiError::internal(error.to_string())
        }
    })?;
    if !canonical.is_dir() {
        return Err(ApiError::bad_request(format!(
            "expected directory path: {}",
            canonical.display()
        )));
    }
    Ok(canonical)
}

fn mime_type_for_image(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "bmp" => "image/bmp",
        "webp" => "image/webp",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use tempfile::TempDir;
    use tower::util::ServiceExt;
    use yoda_config::YoDaSettings;
    use yoda_core::LabelObject;

    use super::{build_api_router, ClassMapResponse, ColorMapResponse, HealthResponse, LabelsResponse, SaveLabelsRequest, TreeNodesResponse};

    fn sample_dataset() -> TempDir {
        let temp = TempDir::new().expect("create temp dir");
        let image_dir = temp.path().join("images/train");
        let label_dir = temp.path().join("labels/train");
        std::fs::create_dir_all(&image_dir).expect("create image dir");
        std::fs::create_dir_all(&label_dir).expect("create label dir");

        let image = image::RgbImage::from_pixel(640, 480, image::Rgb([128, 128, 128]));
        image.save(image_dir.join("test1.jpg")).expect("save jpg");
        image.save(image_dir.join("test2.png")).expect("save png");
        std::fs::write(
            label_dir.join("test1.txt"),
            "0 0.1 0.2 0.3 0.2 0.3 0.8 0.1 0.8 0.05 0.5\n",
        )
        .expect("write labels");
        std::fs::write(label_dir.join("test2.txt"), "1 0.5 0.5 0.4 0.6\n").expect("write labels");

        temp
    }

    fn settings_for(temp: &TempDir) -> YoDaSettings {
        YoDaSettings {
            image_base_path: temp.path().join("images"),
            label_base_path: temp.path().join("labels"),
            class_info: None,
            color_map: None,
            host: Some(String::from("127.0.0.1")),
            port: 8080,
        }
    }

    #[tokio::test]
    async fn health_endpoint_returns_status() {
        let temp = sample_dataset();
        let app = build_api_router(settings_for(&temp)).expect("build router");
        let response = app
            .oneshot(Request::builder().uri("/api/health").body(Body::empty()).expect("request"))
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.expect("body bytes");
        let payload: HealthResponse = serde_json::from_slice(&body).expect("parse health");
        assert_eq!(payload.status, "ok");
    }

    #[tokio::test]
    async fn tree_endpoint_lists_root_nodes() {
        let temp = sample_dataset();
        let app = build_api_router(settings_for(&temp)).expect("build router");
        let response = app
            .oneshot(Request::builder().uri("/api/tree").body(Body::empty()).expect("request"))
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.expect("body bytes");
        let payload: TreeNodesResponse = serde_json::from_slice(&body).expect("parse tree");
        assert_eq!(payload.nodes.len(), 1);
        assert_eq!(payload.nodes[0].label, "train");
    }

    #[tokio::test]
    async fn labels_endpoint_loads_image_metadata_and_labels() {
        let temp = sample_dataset();
        let app = build_api_router(settings_for(&temp)).expect("build router");
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/labels?image_path=train/test1.jpg")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.expect("body bytes");
        let payload: LabelsResponse = serde_json::from_slice(&body).expect("parse labels");
        assert_eq!(payload.width, 640);
        assert_eq!(payload.height, 480);
        assert_eq!(payload.labels.len(), 1);
    }

    #[tokio::test]
    async fn image_endpoint_returns_binary_image_bytes() {
        let temp = sample_dataset();
        let app = build_api_router(settings_for(&temp)).expect("build router");
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/image?image_path=train/test2.png")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("content-type").expect("content type"),
            "image/png"
        );
        let body = to_bytes(response.into_body(), usize::MAX).await.expect("body bytes");
        assert!(!body.is_empty());
    }

    #[tokio::test]
    async fn save_labels_endpoint_persists_updates() {
        let temp = sample_dataset();
        let app = build_api_router(settings_for(&temp)).expect("build router");
        let body = serde_json::to_vec(&SaveLabelsRequest {
            labels: vec![LabelObject {
                index: 0,
                class_id: 9,
                label_type: yoda_core::LabelType::Bbox,
                normalized_coords: vec![0.5, 0.5, 0.4, 0.6],
                pixel_points: vec![yoda_core::Point::new(192.0, 96.0), yoda_core::Point::new(448.0, 384.0)],
                pixel_bbox: yoda_core::PixelBBox::new(192.0, 96.0, 256.0, 288.0),
                visible: true,
            }],
        })
        .expect("serialize body");

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/labels?image_path=train/test2.png")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::OK);
        let written = std::fs::read_to_string(temp.path().join("labels/train/test2.txt"))
            .expect("read labels");
        assert!(written.starts_with("9 "));
    }

    #[tokio::test]
    async fn class_and_color_map_endpoints_return_json() {
        let temp = sample_dataset();
        let app = build_api_router(settings_for(&temp)).expect("build router");

        let class_response = app
            .clone()
            .oneshot(Request::builder().uri("/api/class-map").body(Body::empty()).expect("request"))
            .await
            .expect("response");
        assert_eq!(class_response.status(), StatusCode::OK);
        let class_body = to_bytes(class_response.into_body(), usize::MAX).await.expect("body bytes");
        let classes: ClassMapResponse = serde_json::from_slice(&class_body).expect("parse class map");
        assert!(classes.class_map.is_empty());

        let color_response = app
            .oneshot(Request::builder().uri("/api/color-map").body(Body::empty()).expect("request"))
            .await
            .expect("response");
        assert_eq!(color_response.status(), StatusCode::OK);
        let color_body = to_bytes(color_response.into_body(), usize::MAX).await.expect("body bytes");
        let colors: ColorMapResponse = serde_json::from_slice(&color_body).expect("parse color map");
        assert!(colors.color_map.contains_key(&0));
    }

    #[tokio::test]
    async fn rejects_paths_outside_dataset_root() {
        let temp = sample_dataset();
        let app = build_api_router(settings_for(&temp)).expect("build router");
        std::fs::write(temp.path().join("outside.jpg"), b"x").expect("write outside file");
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/image?image_path=../outside.jpg")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}