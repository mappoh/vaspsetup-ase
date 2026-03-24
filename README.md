# VASPSetup (ASE)

Terminal UI for setting up and submitting VASP calculations to an SGE cluster.

## Installation

Download the latest release from the [Releases page](https://github.com/mappoh/vaspsetup-ase/releases) and extract:

```bash
tar xzf vaspsetup-v0.2.0-linux-x86_64.tar.gz
```

### Requirements

- Python 3.9+ with ASE (`pip install ase`)
- SGE cluster with `qsub`

## Usage

Navigate to a directory containing structure files (POSCAR, .cif, .vasp, .xyz), then run:

```bash
./vaspsetup
```

The `vaspsetup_core/` directory must be alongside the binary.

## Build from source

```bash
cargo build --release
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
