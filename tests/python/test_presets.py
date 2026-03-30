"""Tests for vaspsetup_core.presets — preset loading and listing."""

import pytest

from vaspsetup_core import PresetNotFoundError
from vaspsetup_core.presets import load_preset, list_presets


# All expected preset names
EXPECTED_PRESETS = [
    "bader",
    "charge_density",
    "ci_neb",
    "dimer",
    "elf",
    "frequency",
    "geometry_opt",
    "neb",
    "orbital",
    "pdos",
    "single_point",
]


class TestListPresets:
    """Tests for list_presets()."""

    def test_returns_all_presets(self):
        """All expected presets are listed."""
        result = list_presets()
        assert "presets" in result
        assert sorted(result["presets"]) == EXPECTED_PRESETS

    def test_returns_sorted(self):
        """Preset list is sorted alphabetically."""
        result = list_presets()
        assert result["presets"] == sorted(result["presets"])


class TestLoadPreset:
    """Tests for load_preset()."""

    @pytest.mark.parametrize("name", EXPECTED_PRESETS)
    def test_load_each_preset(self, name):
        """Each preset loads successfully and returns a dict."""
        result = load_preset(name)
        assert isinstance(result, dict)
        assert len(result) > 0

    def test_single_point_params(self):
        """Single point preset has correct key parameters."""
        params = load_preset("single_point")
        assert params["NSW"] == 0
        assert params["IBRION"] == -1
        assert params["LWAVE"] is False

    def test_geometry_opt_params(self):
        """Geometry optimization preset has correct key parameters."""
        params = load_preset("geometry_opt")
        assert params["IBRION"] == 2
        assert params["NSW"] == 500
        assert params["ISIF"] == 2
        assert params["ALGO"] == "Normal"
        assert params["GGA"] == "PE"

    def test_frequency_params(self):
        """Frequency preset has correct key parameters."""
        params = load_preset("frequency")
        assert params["IBRION"] == 5
        assert params["NFREE"] == 2

    def test_bader_params(self):
        """Bader preset enables charge density output."""
        params = load_preset("bader")
        assert params["LCHARG"] is True
        assert params["LAECHG"] is True

    def test_pdos_params(self):
        """PDOS preset has LORBIT and NEDOS."""
        params = load_preset("pdos")
        assert params["LORBIT"] == 11
        assert params["NEDOS"] == 2001
        assert params["ISMEAR"] == -5

    def test_neb_params(self):
        """NEB preset has correct parameters."""
        params = load_preset("neb")
        assert params["IBRION"] == 1
        assert params["IMAGES"] == 5
        assert params["LCLIMB"] is False

    def test_ci_neb_params(self):
        """CI-NEB preset has climbing image enabled."""
        params = load_preset("ci_neb")
        assert params["IBRION"] == 3
        assert params["IMAGES"] == 5
        assert params["LCLIMB"] is True

    def test_dimer_params(self):
        """Dimer preset has no IMAGES."""
        params = load_preset("dimer")
        assert params["IBRION"] == 3
        assert "IMAGES" not in params
        assert "LCLIMB" not in params

    def test_all_presets_have_encut(self):
        """Every preset specifies ENCUT."""
        for name in EXPECTED_PRESETS:
            params = load_preset(name)
            assert "ENCUT" in params, f"Preset '{name}' missing ENCUT"

    def test_nonexistent_preset_raises_error(self):
        """Loading a nonexistent preset raises PresetNotFoundError."""
        with pytest.raises(PresetNotFoundError, match="not found"):
            load_preset("nonexistent_preset")

    def test_error_lists_available_presets(self):
        """Error message includes available preset names."""
        with pytest.raises(PresetNotFoundError, match="single_point"):
            load_preset("nonexistent_preset")

    def test_path_traversal_blocked(self):
        """Path traversal attempts are rejected."""
        with pytest.raises(PresetNotFoundError, match="Invalid preset name"):
            load_preset("../../../etc/passwd")

    def test_path_traversal_with_dots(self):
        """Dotted path traversal is rejected."""
        with pytest.raises(PresetNotFoundError, match="Invalid preset name"):
            load_preset("..%2F..%2Fetc%2Fpasswd")

    def test_empty_name_rejected(self):
        """Empty string preset name is rejected."""
        with pytest.raises(PresetNotFoundError, match="Invalid preset name"):
            load_preset("")

    def test_none_name_rejected(self):
        """None preset name is rejected."""
        with pytest.raises(PresetNotFoundError, match="Invalid preset name"):
            load_preset(None)
