"""Shared I/O utilities for reading atomic structure files."""

from ase.io import read

from vaspsetup_core import StructureReadError


def read_structure(file_path):
    """
    Read an atomic structure file using ASE.

    Args:
        file_path: Path to structure file (POSCAR, CIF, XYZ, etc.)

    Returns:
        ASE Atoms object

    Raises:
        StructureReadError: If the file cannot be read
    """
    try:
        return read(file_path, index=-1)
    except Exception as exc:
        raise StructureReadError(
            f"Cannot read structure from '{file_path}': {exc}"
        ) from exc
