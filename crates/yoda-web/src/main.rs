#![allow(non_snake_case)]

#[cfg(feature = "server")]
use yoda_config::YoDaSettings;
#[cfg(feature = "server")]
use yoda_web::build_router;

#[cfg(feature = "server")]
#[tokio::main]
async fn main() {
    use tracing_subscriber::EnvFilter;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let settings = YoDaSettings::from_env().unwrap_or_default();
    let host = settings
        .host
        .clone()
        .unwrap_or_else(|| dioxus_cli_config::fullstack_address_or_localhost().ip().to_string());
    let address = std::net::SocketAddr::new(
        host.parse().expect("parse YODA_HOST as IP address"),
        settings.port,
    );
    let router = build_router(settings).expect("build yoda-web router");
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("bind yoda-web listener");

    axum::serve(listener, router.into_make_service())
        .await
        .expect("run yoda-web server");
}

#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(yoda_ui::RootApp);
}