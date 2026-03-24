"""Shared pytest fixtures for vaspsetup_core tests."""

import os

import pytest


FIXTURES_DIR = os.path.join(os.path.dirname(__file__), "..", "fixtures")


@pytest.fixture
def poscar_simple():
    """Path to a simple 2-atom Si POSCAR."""
    return os.path.join(FIXTURES_DIR, "POSCAR_simple")


@pytest.fixture
def poscar_fe2o3():
    """Path to a 10-atom Fe2O3 POSCAR (4 Fe, 6 O)."""
    return os.path.join(FIXTURES_DIR, "POSCAR_Fe2O3")


@pytest.fixture
def tmp_output(tmp_path):
    """Temporary directory for VASP output files."""
    return str(tmp_path / "calc_output")
