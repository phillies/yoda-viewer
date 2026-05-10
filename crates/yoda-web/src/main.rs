#![allow(non_snake_case)]

#[cfg(feature = "server")]
use std::net::TcpListener;
#[cfg(feature = "server")]
use std::path::Path;
#[cfg(feature = "server")]
use yoda_config::YoDaSettings;
#[cfg(feature = "server")]
use yoda_web::build_router;

#[cfg(feature = "server")]
#[tokio::main]
async fn main() {
    use tracing_subscriber::EnvFilter;

    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .init();

    let cli_address = dioxus_cli_config::fullstack_address_or_localhost();
    let mut settings = YoDaSettings::from_env().unwrap_or_default();
    let host = settings
        .host
        .clone()
        .unwrap_or_else(|| cli_address.ip().to_string());
    let requested_port = std::env::var("YODA_PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(cli_address.port());
    let port = choose_port(&host, requested_port);
    settings.host = Some(host.clone());
    settings.port = port;
    if port != requested_port {
        tracing::warn!(
            host = %host,
            requested_port,
            selected_port = port,
            "requested yoda-web port was busy; using next available port"
        );
    }
    tracing::info!(
        host = %host,
        port,
        image_root = %settings.image_base_path.display(),
        label_root = %settings.label_base_path.display(),
        class_info = %display_optional_path(settings.class_info.as_deref()),
        color_map = %display_optional_path(settings.color_map.as_deref()),
        "starting yoda-web server"
    );
    let address = std::net::SocketAddr::new(
        host.parse().expect("parse YODA_HOST as IP address"),
        port,
    );
    let router = build_router(settings).expect("build yoda-web router");
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("bind yoda-web listener");

    axum::serve(listener, router.into_make_service())
        .await
        .expect("run yoda-web server");
}

#[cfg(feature = "server")]
fn display_optional_path(path: Option<&Path>) -> String {
    path.map(|path| path.display().to_string())
        .unwrap_or_else(|| String::from("<none>"))
}

#[cfg(feature = "server")]
fn choose_port(host: &str, requested_port: u16) -> u16 {
    if dioxus_cli_config::is_cli_enabled() || std::env::var("YODA_PORT").is_ok() {
        return requested_port;
    }

    for port in requested_port..requested_port.saturating_add(20) {
        if TcpListener::bind((host, port)).is_ok() {
            return port;
        }
    }

    requested_port
}

#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(yoda_ui::RootApp);
}