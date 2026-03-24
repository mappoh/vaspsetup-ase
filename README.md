# VASPSetup (ASE)

Terminal UI for setting up and submitting VASP calculations to an SGE cluster.

## Installation

```bash
uv pip install git+https://github.com/mappoh/vaspsetup-ase.git
```

Or with pip:

```bash
pip install git+https://github.com/mappoh/vaspsetup-ase.git
```

On first run, the binary is automatically downloaded from GitHub Releases.

Requires Python 3.9+ (ASE is installed automatically as a dependency).

## Usage

Navigate to a directory containing structure files (POSCAR, .cif, .vasp, .xyz), then run:

```bash
./vaspsetup
```

## Supported Calculations

- Single Point
- Geometry Optimization
- Frequency Calculation
- Bader Charge
- PDOS
- Charge Density
- Orbital
- Transition State (NEB/CI-NEB/Dimer)

Each supports spin restricted and unrestricted modes.

## Configuration

Cluster settings are stored in `~/.vaspsetup/config.json` (auto-created on first run).

## License

MIT
