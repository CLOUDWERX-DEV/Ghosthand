# Changelog

All notable changes to Ghosthand are in this file. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/).

## [0.1.0] - 2026-11-26

First public release.

### Added

- **Pointer** through a virtual absolute device on `/dev/uinput`: `jump`, `move`, `click`, `dclick`, `rclick`, `mclick`, `down`, `up`, `rdown`, `rup`, `drag`, `rdrag`, `scroll`, `hscroll`, `wait`, `shot`, `pos` and `info`.
- Coordinates are screenshot pixels. The output's size and scale come from niri, or from `GHOSTHAND_OUTPUT_SIZE` and `GHOSTHAND_SCALE` on other compositors.
- Actions are validated up front, so a typo is caught before anything moves. Held buttons are released on exit, on error and on revoke, and the last position is remembered between runs.
- **The gate**: a card with a countdown (Allow, Allow for N min, Deny). Silence is a no, an unreachable overlay is a refusal, and a standing grant lets later requests straight in.
- **Modes** `ask`, `notify` and `off`, with `countdown` and `grant_minutes`. They are read by the overlay from `~/.config/ghosthand/config.json` and can't be set by the program that asks.
- **Cursor tag and ring**: an "Agent Control" tag and a pulsing ring follow the pointer while an agent drives it, and stay on screen at the edges.
- **Panic key**: `ghosthand revoke` stops a running agent, ends any grant and closes the card.
- **Lock check**: refuses to run while the session is locked. Unknown counts as locked. The command can be changed in the config.
- **Overlay**: a standalone Quickshell config, installed with `ghosthand setup` and run with `ghosthand overlay`. Themeable with `accent` and `ui_scale`.
- **MCP server** (`ghosthand mcp`, stdio) with `pointer`, `screenshot`, `screen_info` and `gate_status`. There is no tool to allow a request or switch the gate off.
- `ghosthand doctor` checks uinput access, grim, Quickshell, niri, the overlay and the lock state.
- Packaging: `pyproject.toml`, a udev rule, a systemd user unit, a niri snippet and a PKGBUILD.
- Unit tests for the gate, actions, session and MCP, run in CI on Python 3.9, 3.11 and 3.13. No Python dependencies.
- Docs: setup, configuration, how it works, MCP, other compositors, security and contributing.

[0.1.0]: https://github.com/CLOUDWERX-DEV/ghosthand/releases/tag/v0.1.0
