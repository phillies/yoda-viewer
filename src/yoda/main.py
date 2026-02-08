from pathlib import Path
from typing import Annotated

import typer
from nicegui import ui

from yoda.config import YoDaConfig
from yoda.ui import YoDaBrowser


def start_server(
    dataset_file: Annotated[
        Path | None, typer.Option(help="Path to the dataset YAML file")
    ] = None,
    image_base_path: Annotated[
        Path | None,
        typer.Option(
            help="Base directory for input images. Should contain subdirectories with images."
        ),
    ] = None,
    label_base_path: Annotated[
        Path | None,
        typer.Option(
            help="Base directory for YOLO label files. Should mirror the structure of image_base_path."
        ),
    ] = None,
    class_info: Annotated[
        Path | None,
        typer.Option(
            help="Path to the YAML file containing class information (optional). If provided, it can be used to display class names instead of IDs."
        ),
    ] = None,
    color_map: Annotated[
        Path | None,
        typer.Option(
            help="Path to the YAML file containing color map information (optional). If provided, it can be used to display colors for classes."
        ),
    ] = None,
    host: Annotated[
        str | None,
        typer.Option(
            help="Host address for the NiceGUI server. If not set, it will default to localhost."
        ),
    ] = None,
    port: Annotated[
        int, typer.Option(help="Port number for the NiceGUI server. Defaults to 8080.")
    ] = 8080,
) -> None:
    # In production, we might want to load these paths from environment variables or a config file
    config = YoDaConfig.load()

    yoda_browser = YoDaBrowser(
        config.settings.image_base_path,
        config.settings.label_base_path,
        config.settings.class_info,
    )

    ui.run(
        yoda_browser.render,
        reload=False,
        host=config.settings.host,
        port=config.settings.port,
        title="YoDa Viewer",
    )  # Disable auto-reload in production


def cli_main() -> None:
    typer.run(start_server)


if __name__ in {"__main__", "__mp_main__"}:
    cli_main()
