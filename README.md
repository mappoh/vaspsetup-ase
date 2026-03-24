# VASPSetup (ASE)

Terminal UI for setting up and submitting VASP calculations to an SGE cluster.

## Installation

```bash
uv install vaspsetup-ase
```

Or with pip:

```bash
pip install vaspsetup-ase
```

Requires Python 3.9+ and ASE (installed automatically as a dependency).

Pre-built binaries are also available on the [Releases page](https://github.com/mappoh/vaspsetup-ase/releases).

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
