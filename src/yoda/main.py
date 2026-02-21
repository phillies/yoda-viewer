import socket
from pathlib import Path
from typing import Annotated

import typer
from loguru import logger
from nicegui import ui

from yoda.config import YoDaConfig
from yoda.ui import YoDaBrowser

_MAX_PORT_ATTEMPTS = 20


def _find_available_port(host: str, start_port: int) -> int:
    """Return *start_port* if it is free, otherwise try successive ports.

    Scans up to ``_MAX_PORT_ATTEMPTS`` ports starting from *start_port*.
    Raises ``RuntimeError`` if none are available.
    """
    for offset in range(_MAX_PORT_ATTEMPTS):
        candidate = start_port + offset
        try:
            with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
                sock.bind((host or "0.0.0.0", candidate))
            return candidate
        except OSError:
            logger.info(f"Port {candidate} is in use, trying next…")
    raise RuntimeError(
        f"No available port found in range "
        f"{start_port}–{start_port + _MAX_PORT_ATTEMPTS - 1}"
    )


def start_server(
    dataset_file: Annotated[
        Path | None,
        typer.Option(help="Path to the dataset YAML file"),
    ] = None,
    image_base_path: Annotated[
        Path | None,
        typer.Option(
            help="Base directory for input images. "
            "Should contain subdirectories with images."
        ),
    ] = None,
    label_base_path: Annotated[
        Path | None,
        typer.Option(
            help="Base directory for YOLO label files. "
            "Should mirror the structure of image_base_path."
        ),
    ] = None,
    class_info: Annotated[
        Path | None,
        typer.Option(
            help="Path to the YAML file containing class information (optional)."
        ),
    ] = None,
    color_map: Annotated[
        Path | None,
        typer.Option(
            help="Path to the YAML file containing color map information (optional)."
        ),
    ] = None,
    host: Annotated[
        str | None,
        typer.Option(help="Host address for the NiceGUI server."),
    ] = None,
    port: Annotated[
        int | None,
        typer.Option(help="Port number for the NiceGUI server."),
    ] = None,
) -> None:
    # Build overrides dict from CLI args (only include non-None values)
    overrides: dict[str, object] = {}
    if image_base_path is not None:
        overrides["image_base_path"] = image_base_path
    if label_base_path is not None:
        overrides["label_base_path"] = label_base_path
    if class_info is not None:
        overrides["class_info"] = class_info
    if color_map is not None:
        overrides["color_map"] = color_map
    if host is not None:
        overrides["host"] = host
    if port is not None:
        overrides["port"] = port

    config = YoDaConfig.load(**overrides)

    # When the user did not explicitly set a port, auto-increment if busy.
    effective_port = config.settings.port
    if port is None:
        effective_port = _find_available_port(
            config.settings.host or "0.0.0.0", effective_port
        )
        if effective_port != config.settings.port:
            logger.info(
                f"Using port {effective_port} (default {config.settings.port} was busy)"
            )

    yoda_browser = YoDaBrowser(config)

    ui.run(
        yoda_browser.render,
        reload=False,
        host=config.settings.host,
        port=effective_port,
        title="YoDa Viewer",
    )


def cli_main() -> None:
    typer.run(start_server)


if __name__ in {"__main__", "__mp_main__"}:
    cli_main()
