"""Tests for vaspsetup_core.write — VASP input file generation."""

import os

import pytest

from vaspsetup_core import StructureReadError, WriteError
from vaspsetup_core.write import write_vasp_inputs


class TestWriteVaspInputs:
    """Tests for write_vasp_inputs()."""

    def test_writes_all_three_files(self, poscar_simple, tmp_output):
        """All three VASP input files are created."""
        params = {"ENCUT": 520, "EDIFF": 1e-06, "NSW": 0}
        result = write_vasp_inputs(poscar_simple, tmp_output, params)

        assert os.path.isfile(os.path.join(tmp_output, "POSCAR"))
        assert os.path.isfile(os.path.join(tmp_output, "INCAR"))
        assert os.path.isfile(os.path.join(tmp_output, "KPOINTS"))
        assert result["files_written"] == ["POSCAR", "INCAR", "KPOINTS"]

    def test_incar_contains_params(self, poscar_simple, tmp_output):
        """INCAR file contains the specified parameters."""
        params = {"ENCUT": 520, "EDIFF": 1e-06, "ISMEAR": 0, "SIGMA": 0.05}
        write_vasp_inputs(poscar_simple, tmp_output, params)

        incar_path = os.path.join(tmp_output, "INCAR")
        content = open(incar_path).read()

        assert "ENCUT = 520" in content
        assert "ISMEAR = 0" in content
        assert "SIGMA = 0.05" in content

    def test_incar_boolean_conversion(self, poscar_simple, tmp_output):
        """Python booleans are converted to VASP .TRUE./.FALSE. format."""
        params = {"LWAVE": False, "LCHARG": True}
        write_vasp_inputs(poscar_simple, tmp_output, params)

        content = open(os.path.join(tmp_output, "INCAR")).read()
        assert "LWAVE = .FALSE." in content
        assert "LCHARG = .TRUE." in content

    def test_incar_list_values(self, poscar_simple, tmp_output):
        """List values (like MAGMOM) are written as space-separated."""
        params = {"MAGMOM": [5.0, 5.0, 0.6, 0.6, 0.6]}
        write_vasp_inputs(poscar_simple, tmp_output, params)

        content = open(os.path.join(tmp_output, "INCAR")).read()
        assert "MAGMOM = 5.0  5.0  0.6  0.6  0.6" in content
        # Must NOT contain Python list brackets
        assert "[" not in content
        assert "]" not in content

    def test_incar_excludes_non_incar_keys(self, poscar_simple, tmp_output):
        """VASPSetup-internal keys (pp, kpts) are excluded. GGA is a valid INCAR tag."""
        params = {"ENCUT": 520, "pp": "PBE", "kpts": [4, 4, 4], "GGA": "PE"}
        write_vasp_inputs(poscar_simple, tmp_output, params)

        content = open(os.path.join(tmp_output, "INCAR")).read()
        assert "ENCUT = 520" in content
        assert "pp" not in content
        assert "kpts" not in content
        assert "GGA = PE" in content  # GGA is a valid INCAR tag

    def test_kpoints_default_gamma(self, poscar_simple, tmp_output):
        """Default KPOINTS is Gamma 1x1x1."""
        params = {"ENCUT": 520}
        write_vasp_inputs(poscar_simple, tmp_output, params)

        content = open(os.path.join(tmp_output, "KPOINTS")).read()
        assert "Gamma" in content
        assert "1  1  1" in content

    def test_kpoints_custom_mesh(self, poscar_simple, tmp_output):
        """Custom k-mesh is written correctly."""
        params = {"ENCUT": 520}
        write_vasp_inputs(poscar_simple, tmp_output, params, kpts=(4, 4, 4))

        content = open(os.path.join(tmp_output, "KPOINTS")).read()
        assert "4  4  4" in content

    def test_poscar_preserves_atom_order(self, poscar_fe2o3, tmp_output):
        """POSCAR preserves input atom order (sort=False)."""
        params = {"ENCUT": 520}
        write_vasp_inputs(poscar_fe2o3, tmp_output, params)

        content = open(os.path.join(tmp_output, "POSCAR")).read()
        lines = content.strip().split("\n")
        # Species line should have Fe before O (matching input order)
        species_line = lines[5].strip()
        symbols = species_line.split()
        assert symbols[0] == "Fe"
        assert symbols[1] == "O"

    def test_creates_output_dir(self, poscar_simple, tmp_output):
        """Output directory is created if it doesn't exist."""
        assert not os.path.exists(tmp_output)
        params = {"ENCUT": 520}
        write_vasp_inputs(poscar_simple, tmp_output, params)
        assert os.path.isdir(tmp_output)

    def test_returns_absolute_output_dir(self, poscar_simple, tmp_output):
        """Returned output_dir is an absolute path."""
        params = {"ENCUT": 520}
        result = write_vasp_inputs(poscar_simple, tmp_output, params)
        assert os.path.isabs(result["output_dir"])

    def test_nonexistent_poscar_raises_error(self, tmp_output):
        """Reading a nonexistent structure file raises StructureReadError."""
        with pytest.raises(StructureReadError):
            write_vasp_inputs("/nonexistent/POSCAR", tmp_output, {"ENCUT": 520})

    def test_empty_params_writes_empty_incar(self, poscar_simple, tmp_output):
        """Empty params dict produces an INCAR with only the header comment."""
        write_vasp_inputs(poscar_simple, tmp_output, {})

        content = open(os.path.join(tmp_output, "INCAR")).read()
        lines = [l for l in content.strip().split("\n") if not l.startswith("#")]
        assert len(lines) == 0

    def test_no_temp_dir_left_behind(self, poscar_simple, tmp_output):
        """No vaspsetup_ temp directories remain after successful write."""
        params = {"ENCUT": 520}
        write_vasp_inputs(poscar_simple, tmp_output, params)

        remaining = [
            d for d in os.listdir(tmp_output)
            if d.startswith("vaspsetup_")
        ]
        assert len(remaining) == 0
