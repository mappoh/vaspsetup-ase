# VASPSetup (ASE)

Terminal UI for setting up and submitting VASP calculations to an SGE cluster.

## Requirements

- Rust (for building the TUI binary)
- Python 3.9+ with ASE (`pip install ase`)
- SGE cluster with `qsub`

## Build

```bash
cargo build --release
pip install -e .
```

## Usage

Navigate to a directory containing structure files (POSCAR, .cif, .vasp, .xyz), then run:

```bash
./target/release/vaspsetup
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
