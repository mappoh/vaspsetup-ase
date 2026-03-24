"""VASPSetup Core — Python backend for VASP calculation setup."""

__version__ = "0.2.0"


class CalcFlowError(Exception):
    """Base exception for vaspsetup_core operations."""


class PresetNotFoundError(CalcFlowError):
    """Raised when a requested preset does not exist."""


class StructureReadError(CalcFlowError):
    """Raised when a structure file cannot be read."""


class WriteError(CalcFlowError):
    """Raised when VASP input files cannot be written."""
