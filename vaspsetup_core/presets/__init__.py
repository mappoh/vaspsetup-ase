"""
VASP parameter preset loading.

Presets are JSON files bundled with this package. Each preset contains
default INCAR parameters for a specific calculation type.
"""

import json
import os

from vaspsetup_core import PresetNotFoundError, CalcFlowError


_PRESETS_DIR = os.path.dirname(os.path.abspath(__file__))


def _validate_preset_name(name):
    """Ensure preset name is safe (no path traversal)."""
    if not name or not name.replace("_", "").replace("-", "").isalnum():
        raise PresetNotFoundError(f"Invalid preset name: '{name}'")


def load_preset(name):
    """
    Load a preset by name.

    Args:
        name: Preset name (e.g., "single_point", "geometry_opt")

    Returns:
        dict of VASP parameters

    Raises:
        PresetNotFoundError: If preset file does not exist
        CalcFlowError: If preset file contains invalid JSON
    """
    _validate_preset_name(name)
    filepath = os.path.join(_PRESETS_DIR, f"{name}.json")

    try:
        with open(filepath, encoding="utf-8") as f:
            return json.load(f)
    except FileNotFoundError:
        available = ", ".join(list_presets()["presets"])
        raise PresetNotFoundError(
            f"Preset '{name}' not found. Available: {available}"
        )
    except json.JSONDecodeError as exc:
        raise CalcFlowError(
            f"Preset '{name}' contains invalid JSON: {exc}"
        ) from exc


def list_presets():
    """
    List available preset names.

    Returns:
        dict with "presets" key containing sorted list of names
    """
    presets = [
        f[:-5]
        for f in sorted(os.listdir(_PRESETS_DIR))
        if f.endswith(".json")
    ]
    return {"presets": presets}
