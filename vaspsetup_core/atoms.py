"""
Read atomic structure files and extract species information.

Used by the TUI for:
- MAGMOM setup (species grouped with counts)
- Displaying atom arrangement
"""

from collections import Counter

from vaspsetup_core.io import read_structure


def get_atom_info(file_path):
    """
    Read a structure file and return species information.

    Args:
        file_path: Path to structure file (POSCAR, CIF, XYZ, etc.)

    Returns:
        dict with:
            species: list of {"symbol": str, "count": int} in POSCAR order
            total_atoms: int
    """
    atoms = read_structure(file_path)
    symbols = atoms.get_chemical_symbols()

    # Counter preserves insertion order (Python 3.7+),
    # matching the atom ordering in the structure file
    counts = Counter(symbols)
    species = [{"symbol": s, "count": c} for s, c in counts.items()]

    return {
        "species": species,
        "total_atoms": len(symbols),
    }
