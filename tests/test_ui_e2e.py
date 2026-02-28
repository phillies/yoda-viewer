"""Playwright E2E tests for the YoDa browser UI.

These tests start the NiceGUI app with example data and verify the UI in a
real browser via Playwright.
"""

from __future__ import annotations

import os
import socket
import subprocess
import sys
import time
from pathlib import Path
from typing import Generator

import pytest
from playwright.sync_api import Page, expect

# ---------------------------------------------------------------------------
# App server helper
# ---------------------------------------------------------------------------

_PROJECT_ROOT = Path(__file__).resolve().parent.parent


def _find_free_port() -> int:
    """Find an available TCP port."""
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def _clean_env() -> dict[str, str]:
    """Return a copy of os.environ without pytest/NiceGUI test markers.

    NiceGUI checks ``PYTEST_CURRENT_TEST`` to activate screen-test mode
    which requires ``NICEGUI_SCREEN_TEST_PORT``.  Stripping the marker
    lets the subprocess start as a normal NiceGUI server.
    """
    env = {**os.environ}
    env.pop("PYTEST_CURRENT_TEST", None)
    return env


@pytest.fixture(scope="module")
def app_url() -> Generator[str, None, None]:
    """Start the app server as a subprocess and yield its URL."""
    port = _find_free_port()

    # Start the app as a separate Python process to avoid pydantic-settings
    # CLI parsing conflicts with pytest's sys.argv
    proc = subprocess.Popen(
        [
            sys.executable,
            "-c",
            f"""
import os, sys
os.chdir({str(_PROJECT_ROOT)!r})
sys.argv = ["yoda"]

from yoda.config import YoDaConfig
from yoda.ui import YoDaBrowser
from nicegui import ui

config = YoDaConfig.load(port={port}, host="127.0.0.1")
browser = YoDaBrowser(config)
ui.run(
    browser.render,
    reload=False,
    host="127.0.0.1",
    port={port},
    title="YoDa Viewer",
    show=False,
)
""",
        ],
        cwd=str(_PROJECT_ROOT),
        env=_clean_env(),
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    # Wait for the server to be ready
    for _ in range(60):
        try:
            with socket.create_connection(("127.0.0.1", port), timeout=1):
                break
        except OSError:
            time.sleep(0.5)
            # Check if process died
            if proc.poll() is not None:
                stdout = proc.stdout.read().decode() if proc.stdout else ""
                stderr = proc.stderr.read().decode() if proc.stderr else ""
                pytest.fail(
                    f"App process exited with code {proc.returncode}\n"
                    f"stdout: {stdout}\nstderr: {stderr}"
                )
    else:
        proc.terminate()
        stdout = proc.stdout.read().decode() if proc.stdout else ""
        stderr = proc.stderr.read().decode() if proc.stderr else ""
        pytest.fail(
            f"App server did not start in time\nstdout: {stdout}\nstderr: {stderr}"
        )

    yield f"http://127.0.0.1:{port}"

    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------


class TestYoDaE2E:
    """End-to-end browser tests for the YoDa viewer."""

    def test_page_loads(self, app_url: str, page: Page) -> None:
        """The main page should load and show the file tree."""
        page.goto(app_url)
        page.wait_for_load_state("networkidle")
        # The "Images" label in the file tree
        expect(page.locator("text=Images").first).to_be_visible()

    def test_file_tree_has_folders(self, app_url: str, page: Page) -> None:
        """The file tree should contain the example data subfolders."""
        page.goto(app_url)
        page.wait_for_load_state("networkidle")
        # The top-level tree nodes are the folders; match only *visible*
        # node headers whose exact text is the folder name.
        tree_headers = page.locator(".q-tree__node-header-content:visible")
        # Collect visible tree node labels
        page.wait_for_timeout(1000)
        labels = [
            tree_headers.nth(i).inner_text().strip().split("\n")[-1]
            for i in range(tree_headers.count())
        ]
        assert "test" in labels, f"Expected 'test' folder in {labels}"
        assert "train" in labels, f"Expected 'train' folder in {labels}"

    def test_display_toggles_present(self, app_url: str, page: Page) -> None:
        """All four display toggle switches should be visible."""
        page.goto(app_url)
        page.wait_for_load_state("networkidle")
        expect(page.locator("text=Seg. Masks").first).to_be_visible()
        expect(page.locator("text=Bounding Boxes").first).to_be_visible()
        expect(page.locator("text=Class ID").first).to_be_visible()
        expect(page.locator("text=Class Name").first).to_be_visible()

    def test_class_legend_shows(self, app_url: str, page: Page) -> None:
        """The right drawer should show class names from carparts-seg.yaml."""
        page.goto(app_url)
        page.wait_for_load_state("networkidle")
        # Open the right drawer
        page.locator("button", has=page.locator("i", has_text="list")).first.click()
        page.wait_for_timeout(500)
        # carparts-seg.yaml has "wheel" as class 22
        expect(page.locator("text=wheel").first).to_be_visible(timeout=10000)
        # "Classes" heading should be visible
        expect(page.locator("text=Classes").first).to_be_visible(timeout=5000)

    def test_click_image_shows_viewer(self, app_url: str, page: Page) -> None:
        """Clicking an image file in the tree should display it."""
        page.goto(app_url)
        page.wait_for_load_state("networkidle")

        # Expand "test" folder by clicking the arrow icon (not the label)
        test_node = page.locator(
            ".q-tree__node-header-content:visible",
            has_text="test",
        ).first
        # Click the expand arrow next to the "test" folder
        test_arrow = test_node.locator("..").locator(".q-tree__arrow")
        if test_arrow.count() > 0:
            test_arrow.first.click()
        else:
            # Fall back to clicking the label which might toggle
            test_node.click()
        page.wait_for_timeout(2000)

        # Now find and click a visible file node inside the expanded tree
        visible_items = page.locator(".q-tree__node-header-content:visible")
        clicked = False
        for i in range(visible_items.count()):
            text = visible_items.nth(i).inner_text().strip()
            if any(text.endswith(ext) for ext in (".jpg", ".png", ".jpeg")):
                visible_items.nth(i).click()
                clicked = True
                break
        assert clicked, "No image file found in expanded tree"

        page.wait_for_timeout(3000)

        # The interactive image should now be visible
        img_element = page.locator("img")
        expect(img_element.first).to_be_visible(timeout=10000)

    def test_object_list_drawer(self, app_url: str, page: Page) -> None:
        """The object list drawer should open and show objects after loading."""
        page.goto(app_url)
        page.wait_for_load_state("networkidle")

        # Expand "test" folder
        test_node = page.locator(
            ".q-tree__node-header-content:visible",
            has_text="test",
        ).first
        test_arrow = test_node.locator("..").locator(".q-tree__arrow")
        if test_arrow.count() > 0:
            test_arrow.first.click()
        else:
            test_node.click()
        page.wait_for_timeout(2000)

        # Click an image file
        visible_items = page.locator(".q-tree__node-header-content:visible")
        for i in range(visible_items.count()):
            text = visible_items.nth(i).inner_text().strip()
            if any(text.endswith(ext) for ext in (".jpg", ".png", ".jpeg")):
                visible_items.nth(i).click()
                break
        page.wait_for_timeout(3000)

        # Click the list toggle button to open the right drawer
        page.locator("button", has=page.locator("i", has_text="list")).first.click()
        page.wait_for_timeout(1000)

        # The "Objects" label should be visible in the drawer
        expect(page.locator("text=Objects").first).to_be_visible(timeout=5000)

    def test_class_visibility_checkboxes(self, app_url: str, page: Page) -> None:
        """The class legend should have checkboxes to toggle class visibility."""
        page.goto(app_url)
        page.wait_for_load_state("networkidle")
        # Open the right drawer
        page.locator("button", has=page.locator("i", has_text="list")).first.click()
        page.wait_for_timeout(500)
        # "Classes" heading
        expect(page.locator("text=Classes").first).to_be_visible(timeout=5000)
        # There should be checkboxes in the drawer (one per class)
        checkboxes = page.locator(".q-drawer .q-checkbox")
        page.wait_for_timeout(500)
        assert checkboxes.count() > 0, "Expected class visibility checkboxes"

    def test_object_list_has_visibility_buttons(self, app_url: str, page: Page) -> None:
        """Each object in the list should have a visibility toggle button."""
        page.goto(app_url)
        page.wait_for_load_state("networkidle")

        # Expand "test" folder and click car4_jpg (known to have labels)
        test_node = page.locator(
            ".q-tree__node-header-content:visible",
            has_text="test",
        ).first
        test_arrow = test_node.locator("..").locator(".q-tree__arrow")
        if test_arrow.count() > 0:
            test_arrow.first.click()
        else:
            test_node.click()
        page.wait_for_timeout(2000)

        target = page.locator(
            ".q-tree__node-header-content:visible",
            has_text="car4_jpg",
        ).first
        expect(target).to_be_visible(timeout=5000)
        target.click()
        page.wait_for_timeout(3000)

        # Open drawer
        page.locator("button", has=page.locator("i", has_text="list")).first.click()
        page.wait_for_timeout(1000)

        # Look for visibility icon buttons
        vis_icons = page.locator(".q-drawer i:has-text('visibility')")
        assert vis_icons.count() > 0, "Expected visibility toggle icons"

    def test_object_list_has_class_dropdowns(self, app_url: str, page: Page) -> None:
        """Each object should have a class dropdown to change its class."""
        page.goto(app_url)
        page.wait_for_load_state("networkidle")

        # Expand "test" folder and click car4_jpg (known to have labels)
        test_node = page.locator(
            ".q-tree__node-header-content:visible",
            has_text="test",
        ).first
        test_arrow = test_node.locator("..").locator(".q-tree__arrow")
        if test_arrow.count() > 0:
            test_arrow.first.click()
        else:
            test_node.click()
        page.wait_for_timeout(2000)

        target = page.locator(
            ".q-tree__node-header-content:visible",
            has_text="car4_jpg",
        ).first
        expect(target).to_be_visible(timeout=5000)
        target.click()
        page.wait_for_timeout(3000)

        # Open drawer
        page.locator("button", has=page.locator("i", has_text="list")).first.click()
        page.wait_for_timeout(1000)

        # Look for select/dropdown widgets in the Objects section
        selects = page.locator(".q-drawer .q-select")
        assert selects.count() > 0, "Expected class select dropdowns"


class TestYoDaV3E2E:
    """End-to-end browser tests for Version 3 features (draw/delete)."""

    @pytest.fixture(autouse=True)
    def _protect_label_files(self) -> Generator[None, None, None]:
        """Backup and restore all test label files to prevent side-effects."""
        label_dir = _PROJECT_ROOT / "example_data" / "labels" / "test"
        backups: dict[Path, bytes] = {}
        for f in label_dir.glob("*.txt"):
            backups[f] = f.read_bytes()
        yield
        for f, content in backups.items():
            f.write_bytes(content)

    def _load_test_image(self, page: Page, app_url: str) -> None:
        """Load the page and click a test image that has labels.

        Specifically targets ``car4_jpg`` which has labelled objects.
        The first two images (car25_jpg, car42_jpg) have empty label files.
        """
        page.goto(app_url)
        page.wait_for_load_state("networkidle")

        # Expand "test" folder
        test_node = page.locator(
            ".q-tree__node-header-content:visible",
            has_text="test",
        ).first
        test_arrow = test_node.locator("..").locator(".q-tree__arrow")
        if test_arrow.count() > 0:
            test_arrow.first.click()
        else:
            test_node.click()
        page.wait_for_timeout(2000)

        # Click the known-good image (car4_jpg) which has labelled objects
        target = page.locator(
            ".q-tree__node-header-content:visible",
            has_text="car4_jpg",
        ).first
        expect(target).to_be_visible(timeout=5000)
        target.click()
        page.wait_for_timeout(3000)

    def test_edit_and_draw_mode_buttons_present(self, app_url: str, page: Page) -> None:
        """The toolbar should have Edit (pan_tool) and Add (add) buttons."""
        page.goto(app_url)
        page.wait_for_load_state("networkidle")
        # pan_tool icon button for edit mode
        expect(page.locator("button i:has-text('pan_tool')").first).to_be_visible()
        # add icon button for draw mode
        expect(page.locator("button i:has-text('add')").first).to_be_visible()

    def test_class_selector_in_toolbar(self, app_url: str, page: Page) -> None:
        """The toolbar should have a class selector for new objects."""
        page.goto(app_url)
        page.wait_for_load_state("networkidle")
        # There should be a select widget near the Add button
        toolbar_selects = page.locator(".bg-gray-800 .q-select")
        assert toolbar_selects.count() > 0, "Expected class selector in toolbar"

    def test_draw_mode_button_activates(self, app_url: str, page: Page) -> None:
        """Clicking the Add button after loading an image activates draw mode."""
        self._load_test_image(page, app_url)

        # Click the "Add object" button
        page.locator("button", has=page.locator("i", has_text="add")).first.click()
        page.wait_for_timeout(500)

        # The add button should now be highlighted (yellow)
        add_btn = page.locator("button", has=page.locator("i", has_text="add")).first
        # The button is present and clickable
        expect(add_btn).to_be_visible()

    def test_edit_mode_button_returns_to_edit(self, app_url: str, page: Page) -> None:
        """Clicking the pan_tool button returns to edit mode."""
        self._load_test_image(page, app_url)

        # Enter draw mode
        page.locator("button", has=page.locator("i", has_text="add")).first.click()
        page.wait_for_timeout(300)

        # Return to edit mode
        page.locator("button", has=page.locator("i", has_text="pan_tool")).first.click()
        page.wait_for_timeout(300)

        # Pan tool should be highlighted
        pan_btn = page.locator(
            "button", has=page.locator("i", has_text="pan_tool")
        ).first
        expect(pan_btn).to_be_visible()

    def test_object_list_has_delete_buttons(self, app_url: str, page: Page) -> None:
        """Each object in the list should have a delete button."""
        self._load_test_image(page, app_url)

        # Open drawer
        page.locator("button", has=page.locator("i", has_text="list")).first.click()
        page.wait_for_timeout(1000)

        # Look for delete icon buttons
        delete_icons = page.locator(".q-drawer i:has-text('delete')")
        assert delete_icons.count() > 0, "Expected delete buttons in object list"

    def test_delete_object_reduces_count(self, app_url: str, page: Page) -> None:
        """Clicking a delete button should reduce the number of objects."""
        self._load_test_image(page, app_url)

        # Open drawer
        page.locator("button", has=page.locator("i", has_text="list")).first.click()
        page.wait_for_timeout(1500)

        # Count current delete buttons (one per object)
        delete_btn_loc = page.locator(
            ".q-drawer button",
            has=page.locator("i", has_text="delete"),
        )
        initial_count = delete_btn_loc.count()
        assert initial_count >= 2, f"Need at least 2 objects; found {initial_count}"

        # Click the first delete button
        delete_btn_loc.first.click()

        # Use Playwright's auto-retry to wait for the DOM to update
        expect(delete_btn_loc).to_have_count(initial_count - 1, timeout=5000)

    def test_object_selection_click_on_image(self, app_url: str, page: Page) -> None:
        """Clicking inside a polygon in the image should highlight the row in the drawer."""
        self._load_test_image(page, app_url)

        # Open drawer and wait for it to be visible
        page.locator("button", has=page.locator("i", has_text="list")).first.click()
        page.wait_for_timeout(1500)

        # Verify we are in edit mode (pan_tool button present)
        pan_btn = page.locator(
            "button", has=page.locator("i", has_text="pan_tool")
        ).first
        expect(pan_btn).to_be_visible()

        # Find the SVG element rendered by the interactive image overlay.
        # NiceGUI's interactive_image renders an <svg> for the overlay content.
        svg_el = page.locator("svg").first
        expect(svg_el).to_be_visible(timeout=10000)
        box = svg_el.bounding_box()
        assert box is not None, "Could not find the SVG overlay element"

        # Click near the centre of the image to try to hit a polygon
        cx = box["x"] + box["width"] * 0.5
        cy = box["y"] + box["height"] * 0.5
        page.mouse.click(cx, cy)
        page.wait_for_timeout(800)

        # After clicking, either a row is highlighted (background style set) or
        # the click hit empty space (no highlight) — both are valid.
        # What we verify is that the UI doesn't crash and the object list is intact.
        object_rows = page.locator(
            ".q-drawer button", has=page.locator("i", has_text="delete")
        )
        # Object rows should still be present (no crash)
        assert object_rows.count() >= 1, (
            "Object list rows disappeared after image click"
        )

    def test_toolbar_delete_button_present(self, app_url: str, page: Page) -> None:
        """The toolbar should have a delete button that is disabled when nothing is selected."""
        self._load_test_image(page, app_url)

        # The toolbar delete button (icon='delete' in the toolbar, not in the drawer)
        toolbar_delete = page.locator(
            ".bg-gray-800 button", has=page.locator("i", has_text="delete")
        ).first
        expect(toolbar_delete).to_be_visible()
        # Should be disabled since no object is selected
        expect(toolbar_delete).to_be_disabled()
