"""Unit tests for yoda.config — settings, color map loading, overrides."""

from __future__ import annotations

from pathlib import Path
from unittest.mock import patch

import pytest

from yoda.config import YoDaConfig, YoDaSettings
from yoda.constants import COLOR_MAP


class TestYoDaSettings:
    """Tests for the settings model."""

    def test_defaults(self, monkeypatch: pytest.MonkeyPatch) -> None:
        """Default settings should have expected values."""
        # Clear any YODA_* env vars that may leak from manual runs
        monkeypatch.delenv("YODA_PORT", raising=False)
        monkeypatch.delenv("YODA_HOST", raising=False)
        with patch("sys.argv", ["test"]):
            settings = YoDaSettings()
        assert settings.image_base_path == Path("example_data/images")
        assert settings.label_base_path == Path("example_data/labels")
        assert settings.port == 8080
        assert settings.color_map is None

    def test_env_prefix(self, monkeypatch: pytest.MonkeyPatch) -> None:
        """Settings should read from YODA_ prefixed env vars."""
        monkeypatch.setenv("YODA_PORT", "9999")
        with patch("sys.argv", ["test"]):
            settings = YoDaSettings()
        assert settings.port == 9999


class TestYoDaConfig:
    """Tests for the config singleton."""

    def test_load_creates_config(self) -> None:
        """load() should create and return a YoDaConfig instance."""
        with patch("sys.argv", ["test"]):
            config = YoDaConfig.load()
        assert isinstance(config, YoDaConfig)
        assert config.settings is not None

    def test_load_with_overrides(self) -> None:
        """load() with overrides should apply them."""
        with patch("sys.argv", ["test"]):
            config = YoDaConfig.load(port=1234)
        assert config.settings.port == 1234

    def test_default_color_map_has_100_entries(self) -> None:
        """Without a custom color map, the default 100 colors should be used."""
        with patch("sys.argv", ["test"]):
            config = YoDaConfig.load()
        # Default colors from constants (100 entries)
        assert len(config.color_map_tuples) >= 100
        assert len(config.color_map_strings) >= 100

    def test_custom_color_map_overrides(self, sample_color_map_yaml: Path) -> None:
        """User-provided color map YAML should override defaults."""
        with patch("sys.argv", ["test"]):
            config = YoDaConfig.load(color_map=sample_color_map_yaml)
        # Class 0 should be red from YAML, not the auto-generated color
        assert config.color_map_tuples[0] == (255, 0, 0)
        assert config.get_color_string(0) == "rgb(255,0,0)"
        # Class 1 should be green
        assert config.color_map_tuples[1] == (0, 255, 0)
        # Non-overridden classes should still have defaults
        assert config.color_map_tuples[50] == COLOR_MAP[50]

    def test_get_color_string_fallback(self) -> None:
        """Unknown class IDs should fall back to white."""
        with patch("sys.argv", ["test"]):
            config = YoDaConfig.load()
        assert config.get_color_string(9999) == "rgb(255,255,255)"

    def test_get_color_tuple_fallback(self) -> None:
        """Unknown class IDs should fall back to white tuple."""
        with patch("sys.argv", ["test"]):
            config = YoDaConfig.load()
        assert config.get_color_tuple(9999) == (255, 255, 255)


class TestLoadClassMap:
    """Tests for dataset.load_class_map."""

    def test_load_valid_yaml(self, sample_class_info_yaml: Path) -> None:
        from yoda.dataset import load_class_map

        class_map = load_class_map(sample_class_info_yaml)
        assert class_map == {0: "back_bumper", 1: "front_bumper", 2: "wheel"}

    def test_load_none(self) -> None:
        from yoda.dataset import load_class_map

        assert load_class_map(None) == {}

    def test_load_nonexistent(self, tmp_path: Path) -> None:
        from yoda.dataset import load_class_map

        assert load_class_map(tmp_path / "nope.yaml") == {}
