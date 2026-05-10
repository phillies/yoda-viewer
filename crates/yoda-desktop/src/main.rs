use std::net::TcpListener;
use std::sync::OnceLock;

use anyhow::Context;
use dioxus::prelude::*;
use tracing_subscriber::EnvFilter;
use yoda_config::YoDaSettings;
use yoda_ui::App;
use yoda_web::build_router;

static DESKTOP_API_BASE: OnceLock<String> = OnceLock::new();

fn find_available_port(host: &str, start_port: u16) -> anyhow::Result<u16> {
    for port in start_port..start_port.saturating_add(20) {
        if TcpListener::bind((host, port)).is_ok() {
            return Ok(port);
        }
    }

    anyhow::bail!(
        "no available port found for host {host} in range {}..{}",
        start_port,
        start_port.saturating_add(20)
    )
}

fn desktop_app() -> Element {
    rsx! {
        App { api_base: DESKTOP_API_BASE.get().cloned() }
    }
}

fn main() {
    if let Err(error) = run_desktop() {
        tracing::error!(error = %error, "yoda-desktop failed to start");
        std::process::exit(1);
    }
}

fn run_desktop() -> anyhow::Result<()> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .try_init();

    let mut settings = YoDaSettings::from_env().unwrap_or_default();
    let host = settings
        .host
        .clone()
        .unwrap_or_else(|| String::from("127.0.0.1"));
    let port = find_available_port(&host, settings.port)?;
    settings.host = Some(host.clone());
    settings.port = port;

    let server_settings = settings.clone();
    let server_host = host.clone();
    let api_base = format!("http://{host}:{port}");
    let _ = DESKTOP_API_BASE.set(api_base.clone());

    tracing::info!(
        host = %host,
        port,
        api_base = %api_base,
        image_root = %settings.image_base_path.display(),
        label_root = %settings.label_base_path.display(),
        "starting yoda-desktop backend"
    );

    std::thread::spawn(move || {
        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime,
            Err(error) => {
                tracing::error!(error = %error, "build desktop backend runtime");
                return;
            }
        };

        runtime.block_on(async move {
            let result: anyhow::Result<()> = async {
                let router = build_router(server_settings)
                    .map_err(|error| anyhow::anyhow!("build desktop backend router: {error:?}"))?;
                let listener = tokio::net::TcpListener::bind((server_host.as_str(), port))
                    .await
                    .with_context(|| format!("bind desktop backend listener on {server_host}:{port}"))?;

                axum::serve(listener, router.into_make_service())
                    .await
                    .context("run desktop backend server")?;
                Ok(())
            }
            .await;

            if let Err(error) = result {
                tracing::error!(error = %error, "desktop backend exited with error");
            }
        });
    });

    dioxus::launch(desktop_app);

    Ok(())
}