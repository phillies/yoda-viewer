import os
import secrets
from pathlib import Path

import yaml
from loguru import logger
from pydantic import Field
from pydantic_settings import BaseSettings, SettingsConfigDict

from yoda.constants import COLOR_MAP, COLOR_MAP_RGB_STRING


class YoDaSettings(BaseSettings):
    """Configuration settings for the YoDa application."""

    model_config = SettingsConfigDict(
        env_prefix="YODA_",
        cli_parse_args=True,
        cli_ignore_unknown_args=True,
        extra="ignore",
    )

    image_base_path: Path = Field(
        default=Path("example_data/images"),
        description="Base directory for input images. Should contain subdirectories "
        "with images.",
    )
    label_base_path: Path = Field(
        default=Path("example_data/labels"),
        description="Base directory for YOLO label files. Should mirror the structure "
        "of image_base_path.",
    )
    class_info: Path | None = Field(
        default=Path("example_data/carparts-seg.yaml"),
        description="Path to the YAML file containing class information (optional). "
        "If provided, it can be used to display class names instead of IDs.",
    )
    color_map: Path | None = Field(
        default=None,
        description="Path to the YAML file containing color map information (optional)."
        " If provided, it overrides default auto-generated colors.",
    )
    host: str | None = Field(
        default=None,
        description="Host address for the NiceGUI server. If not set, it will default "
        "to localhost.",
    )
    port: int = Field(
        default=8080,
        description="Port number for the NiceGUI server. Defaults to 8080.",
    )
    storage_secret: str = Field(
        default_factory=secrets.token_hex,
        description="Secret key used to sign the session cookie for user storage. "
        "Set YODA_STORAGE_SECRET to a fixed value to persist sessions across "
        "server restarts.",
    )


class YoDaConfig:
    """Application configuration singleton.

    Manages settings, color map, and provides color lookup for class IDs.
    The color map merges user-provided YAML colors over auto-generated defaults.
    """

    # class attribute
    global_config: "YoDaConfig | None" = None

    # instance attributes
    settings: YoDaSettings
    color_map_tuples: dict[int, tuple[int, int, int]]
    color_map_strings: dict[int, str]

    def __init__(self, **overrides: object) -> None:
        self.settings = YoDaSettings(**overrides)  # type: ignore[arg-type]
        self.color_map_tuples, self.color_map_strings = self._load_color_map()

    @classmethod
    def load(cls, **overrides: object) -> "YoDaConfig":
        """Load or reload the configuration, optionally with CLI overrides."""
        cls.global_config = YoDaConfig(**overrides)
        return cls.global_config

    def _load_color_map(
        self,
    ) -> tuple[dict[int, tuple[int, int, int]], dict[int, str]]:
        """Load the color map from YAML and merge over defaults.

        Returns a tuple of (tuples_map, strings_map).
        """
        # Start with default auto-generated colors
        tuples_map: dict[int, tuple[int, int, int]] = dict(COLOR_MAP)
        strings_map: dict[int, str] = dict(COLOR_MAP_RGB_STRING)

        if self.settings.color_map and self.settings.color_map.exists():
            with self.settings.color_map.open("r") as f:
                color_map_data = yaml.safe_load(f)
            if isinstance(color_map_data, dict):
                for k, v in color_map_data.items():
                    class_id = int(k)
                    if isinstance(v, (list, tuple)) and len(v) == 3:
                        rgb = (int(v[0]), int(v[1]), int(v[2]))
                        tuples_map[class_id] = rgb
                        strings_map[class_id] = f"rgb({rgb[0]},{rgb[1]},{rgb[2]})"
                    elif isinstance(v, str):
                        strings_map[class_id] = v
                logger.info(
                    f"Loaded {len(color_map_data)} custom colors "
                    f"from {self.settings.color_map}"
                )

        return tuples_map, strings_map

    def get_color_tuple(self, class_id: int) -> tuple[int, int, int]:
        """Return the RGB tuple for a given class ID."""
        return self.color_map_tuples.get(class_id, (255, 255, 255))

    def get_color_string(self, class_id: int) -> str:
        """Return the RGB color string for a given class ID."""
        return self.color_map_strings.get(
            class_id,
            "rgb(255,255,255)",
        )
