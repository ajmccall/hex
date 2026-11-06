#!/usr/bin/env python3
"""Preview, or opt in to, this fork's macOS Reword shortcut via Hex's live CLI."""

import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile


MODE_KEY = "dictation_processing.second_shortcut_mode"
KEYS = [
    "dictation_processing.modes",
    MODE_KEY,
    "labs.filler_words",
    "labs.second_dictation_shortcut",
    "second_dictation_hotkey",
]


def cli(binary, *args):
    result = subprocess.run([binary, *args, "--json"], capture_output=True, text=True)
    if result.returncode:
        raise RuntimeError(result.stderr.strip() or result.stdout.strip())
    return json.loads(result.stdout)


def encoded(value):
    if value is None:
        return "none"
    if isinstance(value, str):
        return value
    return json.dumps(value, ensure_ascii=False)


def plan(original, preset, shortcut):
    """Keep all existing modes, prompts, vocabulary, and Global steps untouched."""
    modes = list(original[KEYS[0]])
    existing = [mode for mode in modes if mode.get("name", "").strip().lower() == "reword"]
    if len(existing) > 1:
        raise ValueError("More than one Reword mode exists; give each mode a unique name first")
    if not existing:
        modes.append(preset)
    return {
        KEYS[0]: modes,
        MODE_KEY: existing[0]["name"] if existing else "Reword",
        "labs.filler_words": True,
        "labs.second_dictation_shortcut": True,
        "second_dictation_hotkey": shortcut or original["second_dictation_hotkey"],
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--hex", default="/Applications/Hex.app/Contents/MacOS/hex", help="CLI for the running fork")
    parser.add_argument("--shortcut", help="Second shortcut, e.g. option+space; otherwise keep the existing binding")
    parser.add_argument("--apply", action="store_true", help="Apply the previewed changes; default is read-only")
    args = parser.parse_args()
    # Check the fork-only key first. An older running Hex must fail before any write.
    cli(args.hex, "settings", "get", MODE_KEY)
    original = {key: cli(args.hex, "settings", "get", key)["settings"][0]["value"] for key in KEYS}
    preset = json.loads((Path(__file__).resolve().parent.parent / "resources/reword-mode.json").read_text())
    desired = plan(original, preset, args.shortcut)
    print(json.dumps(desired, indent=2, ensure_ascii=False))
    if not args.apply:
        print("Preview only. Add --apply to save through Hex; your main dictation shortcut is unchanged.")
        return
    if not desired["second_dictation_hotkey"]:
        raise ValueError("Choose --shortcut first, or bind the second shortcut in Hex Settings")
    fd, backup = tempfile.mkstemp(prefix="hex-reword-backup-", suffix=".json")
    with os.fdopen(fd, "w") as stream:
        json.dump(original, stream, indent=2, ensure_ascii=False)
    print("Settings backup:", backup)
    applied = []
    try:
        for key, value in desired.items():
            if value == original[key]:
                continue
            cli(args.hex, "settings", "set", key, encoded(value))
            applied.append(key)
    except Exception:
        for key in reversed(applied):
            try:
                cli(args.hex, "settings", "set", key, encoded(original[key]))
            except Exception as error:
                print(f"Could not restore {key}: {error}. Original values are in {backup}.")
        raise
    print("Reword is ready. The main shortcut keeps normal processing; the second uses Reword.")


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, RuntimeError) as error:
        raise SystemExit(str(error))
