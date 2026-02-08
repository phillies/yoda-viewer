from pathlib import Path

import yaml
from pydantic import Field
from pydantic_settings import BaseSettings, SettingsConfigDict

from yoda.constants import COLOR_MAP_RGB_STRING


class YoDaSettings(BaseSettings):
    """Configuration settings for the YoDa application."""

    model_config = SettingsConfigDict(
        env_prefix="my_prefix_",
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
        default=Path("example_data/color_map.yaml"),
        description="Path to the YAML file containing color map information (optional)."
        " If provided, it can be used to display colors for classes.",
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


class YoDaConfig:
    # class attribute
    global_config: YoDaConfig | None = None

    # instance attributes
    settings: YoDaSettings
    color_map: dict[int, tuple[int, int, int]] = COLOR_MAP_RGB_STRING

    def __init__(self) -> None:
        self.settings = YoDaSettings()
        self.color_map = self.load_color_map()

    @classmethod
    def load(cls) -> YoDaConfig:
        """Loads the configuration settings from environment variables or defaults."""
        if cls.global_config is None:
            cls.global_config = YoDaConfig()
        return cls.global_config

    def load_color_map(self) -> dict[int, str]:
        """Loads the color map from the specified YAML file or uses the default."""
        if self.settings.color_map and self.settings.color_map.exists():
            with self.settings.color_map.open("r") as f:
                color_map_data = yaml.safe_load(f)
                # Assuming the YAML file has a structure like {class_id: "rgb(r,g,b)"}
                return {int(k): v for k, v in color_map_data.items()}

    def get_color_string(self, class_id: int) -> str:
        """Returns the RGB color string for a given class ID."""
        color = self.color_map.get(class_id, (255, 255, 255))  # Default to white
        return f"rgb({color[0]},{color[1]},{color[2]})"
