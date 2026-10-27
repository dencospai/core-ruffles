"""Check startup and cached-frame lifetime for each local SWF (not a playthrough)."""
import argparse
import json
from pathlib import Path
import subprocess
import sys


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("library")
    parser.add_argument("games")
    parser.add_argument("--output", required=True)
    parser.add_argument("--frames", type=int, default=180)
    args = parser.parse_args()
    root = Path(args.output).resolve()
    root.mkdir(parents=True, exist_ok=True)
    games = sorted(Path(args.games).resolve().glob("*.swf"))
    games += list(Path(args.games).resolve().glob("Kasumi Rebirth/FFHC.swf"))
    results = []
    for game in games:
        output = root / game.stem
        output.mkdir(exist_ok=True)
        command = [sys.executable, str(Path(__file__).with_name("probe_game.py")),
                   str(Path(args.library).resolve()), str(game), "--output", str(output),
                   "--frames", str(args.frames), "--capture", str(args.frames), "--check-cache"]
        with (output / "process.log").open("w", encoding="utf-8") as log:
            try:
                process = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, timeout=180)
                code = process.returncode
            except subprocess.TimeoutExpired:
                code = "timeout"
        results.append({"game": str(game.relative_to(Path(args.games).resolve())), "exit_code": code})
        print(results[-1], flush=True)
        (root / "matrix.json").write_text(json.dumps(results, indent=2) + "\n")
    assert all(item["exit_code"] == 0 for item in results), "Some startup/cache probes failed"


if __name__ == "__main__":
    main()
