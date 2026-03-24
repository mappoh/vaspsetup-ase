"""
CLI dispatcher for vaspsetup_core.

Rust TUI communicates with this module via subprocess + JSON:

    echo '{"command": "atoms", "args": {"file": "/path/to/POSCAR"}}' | python -m vaspsetup_core

Response (stdout):
    {"status": "ok", "data": {...}}
    {"status": "error", "message": "..."}

Commands:
    atoms   — Read structure file, return species and counts
    write   — Write POSCAR/INCAR/KPOINTS to output directory
    preset  — Load a preset or list available presets
    version — Return package version
"""

import json
import sys

from vaspsetup_core import __version__, CalcFlowError


def _require_args(args, command, *keys):
    """Validate that required keys are present in args dict."""
    for key in keys:
        if key not in args:
            raise CalcFlowError(f"'{command}' command requires '{key}' in args")


def _dispatch(command, args):
    """Route command to the appropriate module function."""
    if command == "atoms":
        _require_args(args, command, "file")
        from vaspsetup_core.atoms import get_atom_info
        return get_atom_info(args["file"])

    if command == "write":
        _require_args(args, command, "file", "output_dir", "params")
        from vaspsetup_core.write import write_vasp_inputs
        return write_vasp_inputs(
            poscar_path=args["file"],
            output_dir=args["output_dir"],
            params=args["params"],
            kpts=tuple(args.get("kpts", [1, 1, 1])),
        )

    if command == "preset":
        from vaspsetup_core.presets import load_preset, list_presets
        if args.get("list"):
            return list_presets()
        _require_args(args, command, "name")
        return load_preset(args["name"])

    if command == "version":
        return {"version": __version__}

    raise CalcFlowError(f"Unknown command: {command}")


def main():
    """Read JSON request from stdin, dispatch, write JSON response to stdout."""
    try:
        raw = sys.stdin.read()
        if not raw.strip():
            raise CalcFlowError("No input received on stdin")

        request = json.loads(raw)
        command = request.get("command")
        args = request.get("args", {})

        if not command:
            raise CalcFlowError("Missing 'command' field in request")

        result = _dispatch(command, args)
        response = {"status": "ok", "data": result}

    except json.JSONDecodeError as exc:
        response = {"status": "error", "message": f"Invalid JSON input: {exc}"}
        print(json.dumps(response), flush=True)
        sys.exit(1)

    except CalcFlowError as exc:
        response = {"status": "error", "message": str(exc)}
        print(json.dumps(response), flush=True)
        sys.exit(1)

    except Exception as exc:
        response = {"status": "error", "message": f"Unexpected error: {type(exc).__name__}: {exc}"}
        print(json.dumps(response), flush=True)
        sys.exit(1)

    print(json.dumps(response), flush=True)


if __name__ == "__main__":
    main()
