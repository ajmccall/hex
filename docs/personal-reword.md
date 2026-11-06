# Personal fork: optional Reword dictation

Repository: <https://github.com/ajmccall/hex>, forked from `anomalyco/hex`.
This is a personal macOS feature, not an upstream release.

## Contract

- The main dictation shortcut keeps ordinary app/site-selected processing.
- An optional second shortcut explicitly selects a named mode such as **Reword**, regardless of the foreground app or site. This is per dictation, not a sticky global switch.
- Shared filler removal and Global replacements still run first on both paths. Raw dictation remains a separate, fully unprocessed bypass.
- Reword turns rambling dictation into a concise, natural message for Slack or similar communication. Preserve meaning, details, nuance, and certainty; remove filler and repetition; add no facts; no em dashes. Paste only the final message, never questions or commentary.
- Use Hex's fast default connected OpenCode model, with a 30-second deadline. On failure or timeout, keep the cleaned transcript, as existing rewriting does.
- Existing profiles, vocabulary, shortcuts, and Global processing are preserved. Nothing opts in automatically.

## What changed

`dictation_processing.second_shortcut_mode` is an optional string:

- `none` / JSON `null` (default): the second shortcut uses automatic app/site selection, as before.
- `Global`: explicitly use only Global processing, ignoring app/site modes.
- A unique mode name, such as `Reword`: explicitly use that mode. Modes without `applications` or `browser_hosts` work here too.

The setting is in **General → Processing**, immediately below **Second dictation shortcut**, while **Labs → Second dictation shortcut** is on. It also works through `hex settings set dictation_processing.second_shortcut_mode Reword`.
The initiating shortcut latches the name; releases, double-tap locking, CLI stop/print, and queued jobs retain it. A missing or ambiguous name falls back to Global processing rather than an unrelated app's rewrite. CLI assignment rejects missing/ambiguous names. The profile's steps, like automatic modes, are read when processing starts; changing a profile can therefore affect queued jobs.

The second shortcut is macOS-only, like the existing lab it extends. Linux and Windows retain their existing app/site processing and raw bypass. The feature is off until the existing lab is enabled and a second shortcut is bound. No new microphone capture, subprocess, provider, or paste path is introduced.

## Build and opt in

Build as a local copy so upstream automatic updates cannot replace the fork:

```sh
HEX_LOCAL_BUILD=1 HEX_RELEASE_UNSIGNED=1 scripts/build-app.sh
```

This creates an ad-hoc signed `target/app/Hex.app`; it does not replace `/Applications/Hex.app`. For Developer ID signing instead, omit `HEX_RELEASE_UNSIGNED=1` and set `HEX_TEAM_ID`. Keep the original app as a backup. Quit the original before launching the fork; both otherwise share the same data directory and application identity. A differently signed build may need Accessibility permission granted again.

Once the fork is running, preview the setup:

```sh
python3 scripts/configure-reword.py --hex target/app/Hex.app/Contents/MacOS/hex --shortcut option+space
```

Then repeat with `--apply`. Choose another shortcut if Hex reports a conflict. The script talks only to the running app through `hex settings`, never edits its settings file, checks the fork-only setting before writing, saves the old values in an owner-only temporary backup, and attempts rollback on failure. Without `--apply` it only reads. A pre-existing Reword mode and its custom prompt are kept unchanged.

Setup enables the existing **Remove filler words** lab (`um`, `umm`, `uh`, `uhh`, `erm`, `er`, `ah`), adds `resources/reword-mode.json` if needed, selects it for the second shortcut, and binds that shortcut. It does not change the main shortcut, Global rewrite, replacements, or vocabulary. For the current personal configuration, Fn stays Fn.

An already running, connected OpenCode service is required for rewriting. Speech transcription and filler removal stay local; Reword sends text/context to the connected provider. Without the service or within a failed deadline, the cleaned transcript still pastes.

To go back to automatic processing on the second key:

```sh
hex settings set dictation_processing.second_shortcut_mode none
```

To disable that shortcut entirely: `hex labs off second-dictation-shortcut`.

## Checks and proof limits

- `scripts/check dictation::config`: explicit modes without app rules, Global corrections first, manual precedence over websites, missing-mode recovery, the real preset's cleaned fallback without any provider call.
- `scripts/check second_shortcut`: optional-setting validation, mode latching, main/raw starts clearing the override, queued context independence.
- `scripts/check dictation::processor`: existing ordering, model selection, cancellation, and deadline behavior.
- `python3 scripts/test-configure-reword.py`: read-only preview safety, preserving customized modes/shortcuts, idempotence, legacy/unsupported-app refusal, private backup and rollback after a shortcut conflict.
- `scripts/check rune_ui`: the simulated macOS selector's choices, main-key preservation, lab/platform visibility, and light/dark `mac-settings-reword` goldens on Linux. Linux renders the processing selector; the existing shortcut recorder itself remains macOS-only.

Executed locally on macOS: `scripts/check` passed all 2,232 non-ignored tests (31 opt-in tests skipped), including strict Clippy, formatting, code-shape and Nix-pin checks. The setup script's eight tests passed. FFmpeg is required for the existing capture-core media tests. The new selector was also rendered and inspected in both appearances on macOS.

Headless previews and unit tests are not proof of a physical shortcut, a real provider rewrite, or a paste into Slack. Those need a smoke test after launching the local build. No test should launch a second microphone listener beside the installed app or rewrite the user's live settings.
