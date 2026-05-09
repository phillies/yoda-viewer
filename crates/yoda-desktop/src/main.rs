use std::net::TcpListener;
use std::sync::OnceLock;

use dioxus::prelude::*;
use yoda_config::YoDaSettings;
use yoda_ui::App;
use yoda_web::build_router;

static DESKTOP_API_BASE: OnceLock<String> = OnceLock::new();

fn find_available_port(host: &str, start_port: u16) -> u16 {
    for port in start_port..start_port.saturating_add(20) {
        if TcpListener::bind((host, port)).is_ok() {
            return port;
        }
    }

    panic!("no available port found starting at {start_port}")
}

fn desktop_app() -> Element {
    rsx! {
        App { api_base: DESKTOP_API_BASE.get().cloned() }
    }
}

fn main() {
    let mut settings = YoDaSettings::from_env().unwrap_or_default();
    let host = settings
        .host
        .clone()
        .unwrap_or_else(|| String::from("127.0.0.1"));
    let port = find_available_port(&host, settings.port);
    settings.host = Some(host.clone());
    settings.port = port;

    let server_settings = settings.clone();
    let server_host = host.clone();
    let api_base = format!("http://{host}:{port}");
    let _ = DESKTOP_API_BASE.set(api_base.clone());

    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("build desktop backend runtime");

        runtime.block_on(async move {
            let router = build_router(server_settings).expect("build desktop backend router");
            let listener = tokio::net::TcpListener::bind((server_host.as_str(), port))
                .await
                .expect("bind desktop backend listener");

            axum::serve(listener, router.into_make_service())
                .await
                .expect("run desktop backend server");
        });
    });

    dioxus::launch(desktop_app);
}