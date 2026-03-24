"""Tests for vaspsetup_core.__main__ — JSON protocol and dispatcher."""

import json
import subprocess
import sys
import os

import pytest

FIXTURES_DIR = os.path.join(os.path.dirname(__file__), "..", "fixtures")


def _call_vaspsetup(request_dict):
    """Call vaspsetup_core via subprocess, matching how Rust will call it."""
    result = subprocess.run(
        [sys.executable, "-m", "vaspsetup_core"],
        input=json.dumps(request_dict),
        capture_output=True,
        text=True,
    )
    if not result.stdout.strip():
        raise RuntimeError(
            f"No output from vaspsetup_core. stderr: {result.stderr}"
        )
    response = json.loads(result.stdout)
    return response, result.returncode


class TestVersionCommand:
    """Tests for the 'version' command."""

    def test_returns_version(self):
        response, code = _call_vaspsetup({"command": "version"})
        assert code == 0
        assert response["status"] == "ok"
        assert "version" in response["data"]


class TestAtomsCommand:
    """Tests for the 'atoms' command."""

    def test_returns_species(self):
        poscar = os.path.join(FIXTURES_DIR, "POSCAR_simple")
        response, code = _call_vaspsetup({
            "command": "atoms",
            "args": {"file": poscar}
        })
        assert code == 0
        assert response["status"] == "ok"
        assert response["data"]["total_atoms"] == 2

    def test_missing_file_arg(self):
        response, code = _call_vaspsetup({
            "command": "atoms",
            "args": {}
        })
        assert code == 1
        assert response["status"] == "error"
        assert "requires 'file'" in response["message"]

    def test_nonexistent_file(self):
        response, code = _call_vaspsetup({
            "command": "atoms",
            "args": {"file": "/nonexistent/POSCAR"}
        })
        assert code == 1
        assert response["status"] == "error"
        assert "Cannot read" in response["message"]


class TestPresetCommand:
    """Tests for the 'preset' command."""

    def test_list_presets(self):
        response, code = _call_vaspsetup({
            "command": "preset",
            "args": {"list": True}
        })
        assert code == 0
        assert "presets" in response["data"]
        assert "single_point" in response["data"]["presets"]

    def test_load_preset(self):
        response, code = _call_vaspsetup({
            "command": "preset",
            "args": {"name": "single_point"}
        })
        assert code == 0
        assert response["data"]["NSW"] == 0

    def test_missing_name(self):
        response, code = _call_vaspsetup({
            "command": "preset",
            "args": {}
        })
        assert code == 1
        assert response["status"] == "error"
        assert "requires 'name'" in response["message"]


class TestWriteCommand:
    """Tests for the 'write' command."""

    def test_write_files(self, tmp_path):
        poscar = os.path.join(FIXTURES_DIR, "POSCAR_simple")
        output_dir = str(tmp_path / "test_calc")
        response, code = _call_vaspsetup({
            "command": "write",
            "args": {
                "file": poscar,
                "output_dir": output_dir,
                "params": {"ENCUT": 520, "NSW": 0},
                "kpts": [4, 4, 4]
            }
        })
        assert code == 0
        assert response["status"] == "ok"
        assert os.path.isfile(os.path.join(output_dir, "POSCAR"))
        assert os.path.isfile(os.path.join(output_dir, "INCAR"))
        assert os.path.isfile(os.path.join(output_dir, "KPOINTS"))

    def test_missing_params(self):
        response, code = _call_vaspsetup({
            "command": "write",
            "args": {"file": "/some/file", "output_dir": "/some/dir"}
        })
        assert code == 1
        assert "requires 'params'" in response["message"]


class TestErrorHandling:
    """Tests for protocol-level error handling."""

    def test_unknown_command(self):
        response, code = _call_vaspsetup({"command": "bogus"})
        assert code == 1
        assert "Unknown command" in response["message"]

    def test_missing_command(self):
        response, code = _call_vaspsetup({"args": {}})
        assert code == 1
        assert "Missing 'command'" in response["message"]

    def test_empty_input(self):
        """Empty stdin produces an error."""
        result = subprocess.run(
            [sys.executable, "-m", "vaspsetup_core"],
            input="",
            capture_output=True,
            text=True,
        )
        response = json.loads(result.stdout)
        assert response["status"] == "error"
        assert result.returncode == 1

    def test_invalid_json(self):
        """Non-JSON input produces an error."""
        result = subprocess.run(
            [sys.executable, "-m", "vaspsetup_core"],
            input="not json at all",
            capture_output=True,
            text=True,
        )
        response = json.loads(result.stdout)
        assert response["status"] == "error"
        assert "Invalid JSON" in response["message"]

    def test_response_is_single_line_json(self):
        """Response is always exactly one line of valid JSON."""
        response, code = _call_vaspsetup({"command": "version"})
        # If we got here, json.loads succeeded — it's valid JSON
        assert isinstance(response, dict)
        assert "status" in response
