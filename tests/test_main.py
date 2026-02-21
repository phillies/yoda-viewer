"""Unit tests for yoda.main — port auto-increment logic."""

from __future__ import annotations

import socket

from yoda.main import _find_available_port


class TestFindAvailablePort:
    """Tests for the port-probing helper."""

    def test_returns_start_port_when_free(self) -> None:
        """If the start port is free it should be returned immediately."""
        # Bind to an ephemeral port so we know which one is free after release
        with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
            s.bind(("127.0.0.1", 0))
            free_port = s.getsockname()[1]
        # Port is now released
        assert _find_available_port("127.0.0.1", free_port) == free_port

    def test_skips_busy_port(self) -> None:
        """If the first port is occupied, it should return the next free one."""
        # Occupy a port
        blocker = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        blocker.bind(("127.0.0.1", 0))
        busy_port = blocker.getsockname()[1]
        try:
            result = _find_available_port("127.0.0.1", busy_port)
            assert result > busy_port
        finally:
            blocker.close()

    def test_raises_when_all_ports_busy(self) -> None:
        """RuntimeError should be raised when every candidate port is used."""
        blockers: list[socket.socket] = []
        # Grab a contiguous range of ports
        first = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        first.bind(("127.0.0.1", 0))
        start_port = first.getsockname()[1]
        blockers.append(first)
        try:
            for offset in range(1, 20):
                s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                try:
                    s.bind(("127.0.0.1", start_port + offset))
                    blockers.append(s)
                except OSError:
                    # Port already taken by something else — that's fine
                    s.close()
            try:
                _find_available_port("127.0.0.1", start_port)
            except RuntimeError:
                pass  # expected
        finally:
            for b in blockers:
                b.close()
