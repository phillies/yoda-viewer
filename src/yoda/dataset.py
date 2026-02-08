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
        """Loads image and label file paths from the specified directories and optionally loads class information from a YAML file.
        Arguments:
            image_base_path (Path): The base directory containing image files. Should have a structure with subdirectories for different classes or categories.
            label_base_path (Path): The base directory containing YOLO label files. Should mirror the structure of image_base_path, with corresponding .txt files for each image.
            class_info_yaml (Path | None): Optional path to a YAML file containing class information. If provided, it should have a "names" key mapping class IDs to class names.
        """
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
