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
use yoda_app::AppServices;
use yoda_app::RepositoryBackedAppServices;
use yoda_config::YoDaSettings;
use yoda_core::LabelObject;
use yoda_data::{DatasetRepository, LocalDatasetRepository, TreeNode};
use yoda_ui::RootApp;

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
    ensure_public_dir()?;
    let state = Arc::new(BackendState::from_settings(settings)?);
    Ok(Router::<FullstackState>::new()
        .nest("/api", api_router(state.clone()))
        .serve_dioxus_application(ServeConfig::new(), RootApp))
}

pub fn build_api_router(settings: YoDaSettings) -> Result<Router, ApiError> {
    let state = Arc::new(BackendState::from_settings(settings)?);
    Ok(Router::<FullstackState>::new()
        .nest("/api", api_router(state))
        .serve_api_application(ServeConfig::new(), RootApp))
}

fn api_router(state: Arc<BackendState>) -> Router<FullstackState> {
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

fn ensure_public_dir() -> Result<(), ApiError> {
    let executable = std::env::current_exe()?;
    let Some(parent) = executable.parent() else {
        return Err(ApiError::internal("unable to determine executable directory"));
    };

    fs::create_dir_all(parent.join("public"))?;
    Ok(())
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: String::from("ok"),
        version: String::from(env!("CARGO_PKG_VERSION")),
    })
}

async fn list_tree(Extension(state): Extension<Arc<BackendState>>) -> Result<Json<TreeNodesResponse>, ApiError> {
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