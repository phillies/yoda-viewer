#![allow(non_snake_case)]

use yoda_ui::App;

#[cfg(feature = "server")]
#[tokio::main]
async fn main() {
    use dioxus_server::{DioxusRouterExt, ServeConfig};
    use tracing_subscriber::EnvFilter;

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let address = dioxus_cli_config::fullstack_address_or_localhost();
    let router = axum::Router::new().serve_dioxus_application(ServeConfig::new(), App);
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .expect("bind yoda-web listener");

    axum::serve(listener, router.into_make_service())
        .await
        .expect("run yoda-web server");
}

#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(App);
}