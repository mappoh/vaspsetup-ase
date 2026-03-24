"""Tests for vaspsetup_core.atoms — structure reading and species extraction."""

import pytest

from vaspsetup_core import StructureReadError
from vaspsetup_core.atoms import get_atom_info


class TestGetAtomInfo:
    """Tests for get_atom_info()."""

    def test_simple_single_species(self, poscar_simple):
        """Read a single-species structure (Si2)."""
        result = get_atom_info(poscar_simple)

        assert result["total_atoms"] == 2
        assert len(result["species"]) == 1
        assert result["species"][0]["symbol"] == "Si"
        assert result["species"][0]["count"] == 2

    def test_multi_species_ordering(self, poscar_fe2o3):
        """Read a multi-species structure (Fe2O3) — species in POSCAR order."""
        result = get_atom_info(poscar_fe2o3)

        assert result["total_atoms"] == 10
        assert len(result["species"]) == 2

        # Fe comes first in the POSCAR, then O
        assert result["species"][0]["symbol"] == "Fe"
        assert result["species"][0]["count"] == 4
        assert result["species"][1]["symbol"] == "O"
        assert result["species"][1]["count"] == 6

    def test_species_counts_sum_to_total(self, poscar_fe2o3):
        """Species counts must sum to total_atoms."""
        result = get_atom_info(poscar_fe2o3)
        total = sum(s["count"] for s in result["species"])
        assert total == result["total_atoms"]

    def test_nonexistent_file_raises_error(self):
        """Reading a nonexistent file raises StructureReadError."""
        with pytest.raises(StructureReadError, match="Cannot read structure"):
            get_atom_info("/nonexistent/POSCAR")

    def test_return_type(self, poscar_simple):
        """Return value is a dict with expected keys."""
        result = get_atom_info(poscar_simple)
        assert isinstance(result, dict)
        assert "species" in result
        assert "total_atoms" in result
        assert isinstance(result["species"], list)
        assert isinstance(result["total_atoms"], int)
