#!/usr/bin/env python3
"""No app, microphone, settings writes, or provider calls."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("configure_reword", Path(__file__).with_name("configure-reword.py"))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ConfigureReword(unittest.TestCase):
    def original(self):
        return {
            module.KEYS[0]: [{"name": "Editor", "vocabulary": ["Rune"]}],
            module.MODE_KEY: None,
            "labs.filler_words": False,
            "labs.second_dictation_shortcut": False,
            "second_dictation_hotkey": None,
        }

    def test_adds_reword_without_changing_existing_modes(self):
        original = self.original()
        desired = module.plan(original, {"name": "Reword"}, "f6")
        self.assertEqual(desired[module.KEYS[0]][0], original[module.KEYS[0]][0])
        self.assertEqual(len(original[module.KEYS[0]]), 1)
        self.assertEqual(desired[module.MODE_KEY], "Reword")
        self.assertEqual(desired["second_dictation_hotkey"], "f6")
        self.assertTrue(desired["labs.filler_words"])

    def test_preserves_custom_reword_and_existing_shortcut(self):
        original = self.original()
        custom = {"name": "reword", "post_processing": {"prompt": "My prompt"}}
        original[module.KEYS[0]].append(custom)
        original["second_dictation_hotkey"] = "f6"
        desired = module.plan(original, {"name": "Reword"}, None)
        self.assertEqual(desired[module.KEYS[0]][-1], custom)
        self.assertEqual(desired[module.MODE_KEY], "reword")
        self.assertEqual(desired["second_dictation_hotkey"], "f6")
        self.assertEqual(module.plan(desired, {"name": "Reword"}, None), desired)

    def test_duplicate_mode_names_are_refused(self):
        original = self.original()
        original[module.KEYS[0]] = [{"name": "Reword"}, {"name": " reword "}]
        with self.assertRaises(ValueError):
            module.plan(original, {"name": "Reword"}, "f6")

    def test_failure_rolls_back_every_successful_change(self):
        original = self.original()
        writes = []

        def cli(binary, action, operation, key, *value):
            if operation == "get":
                return {"settings": [{"value": original[key]}]}
            writes.append((key, value[0]))
            if key == "second_dictation_hotkey":
                raise RuntimeError("shortcut-in-use")
            return {}

        with tempfile.TemporaryDirectory() as directory:
            real_mkstemp = tempfile.mkstemp
            with patch.object(module, "cli", cli), patch.object(module.tempfile, "mkstemp", lambda **kwargs: real_mkstemp(dir=directory, **kwargs)), patch("sys.argv", ["configure-reword.py", "--apply", "--shortcut", "f6"]), contextlib.redirect_stdout(io.StringIO()):
                with self.assertRaisesRegex(RuntimeError, "shortcut-in-use"):
                    module.main()
            backup = next(Path(directory).iterdir())
            self.assertEqual(json.loads(backup.read_text()), original)
            self.assertEqual(backup.stat().st_mode & 0o777, 0o600)
        expected = [(key, module.encoded(original[key])) for key in reversed(module.KEYS[:-1])]
        self.assertEqual(writes[-4:], expected)

    def test_old_hex_fails_before_writing_anything(self):
        with patch.object(module, "cli", side_effect=RuntimeError("unknown-setting")) as cli, patch("sys.argv", ["configure-reword.py", "--apply"]):
            with self.assertRaises(RuntimeError):
                module.main()
        self.assertEqual(cli.call_count, 1)
        self.assertEqual(cli.call_args.args[2], "get")

    def test_preset_uses_fast_default_model_and_a_deadline(self):
        preset = json.loads((Path(__file__).resolve().parent.parent / "resources/reword-mode.json").read_text())
        self.assertFalse(preset["applications"])
        self.assertFalse(preset["browser_hosts"])
        self.assertNotIn("model", preset["post_processing"])
        self.assertEqual(preset["post_processing"]["deadline_seconds"], 30)


if __name__ == "__main__":
    unittest.main()
