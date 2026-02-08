from pathlib import Path

import yaml
from loguru import logger

from yoda.fileops import get_files


class YoDa:
    image_paths: list[Path]
    label_paths: list[Path]
    class_map: dict[int, str]

    def load_folders(
        self,
        image_base_path: Path,
        label_base_path: Path,
        class_info_yaml: Path | None = None,
    ) -> None:
        self.image_paths = get_files(image_base_path)
        self.label_paths = get_files(label_base_path)
        logger.info(f"Loaded {len(self.image_paths)} images from {image_base_path}")
        logger.info(
            f"Loaded {len(self.label_paths)} label files from {label_base_path}"
        )
        if class_info_yaml and class_info_yaml.exists():
            with class_info_yaml.open("r", encoding="utf-8") as f:
                self.class_map = yaml.safe_load(f).get("names", {})
        else:
            self.class_map = {}

    def load_dataset(self, dataset_yaml: Path) -> None: ...
