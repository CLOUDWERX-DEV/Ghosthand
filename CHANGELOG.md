# Changelog

All notable changes to Ghosthand are in this file. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/).

## [0.2.0] - 2026-11-26

Keyboard support, works beyond niri, speaks the current MCP spec, and ships an agent skill.

### Added

- **Keyboard**, through a second virtual device (`ghosthand-keyboard`): `key` (shortcuts and chords like `ctrl+shift+t`, sequences like `Tab Tab Return`), `keydown` and `keyup` (hold Shift while you click), `type` (text, with `\n` and `\t`) and `paste` (clipboard plus Ctrl+V, restoring the clipboard, for text `type` can't make).
- **Your real layout**: `type` and single-character keys are resolved through the layout with libxkbcommon (via ctypes, so still no Python dependencies), so QWERTY, AZERTY, Dvorak, German and the rest work, including Shift and AltGr characters. The layout is detected from the config, the environment, `setxkbmap` or `localectl`, with a built-in US layout as the fallback.
- **`focus`** prints the focused window (niri, Hyprland, sway, X11) so an agent knows where its keys will land. It needs no approval.
- **A gate that knows about the keyboard**: the card says "mouse", "keyboard" or "mouse and keyboard" and shows exactly what will be typed or pressed. A standing "Allow for N min" for the mouse does not cover the keyboard, they're granted separately. This works in the overlay and in the dialog fallback.
- Keyboard config: `keyboard` (false switches it off), `keyboard_layout`, `keyboard_variant`, `keyboard_options`, `type_delay_ms` and `paste_terminal`.
- Held keys and buttons are always released at the end, on error and on revoke, and a revoke stops typing within a couple of dozen characters. Typed text is checked against the layout before anything is pressed.
- **Screenshots on any session**: grim (niri, Hyprland, sway, river), maim, scrot or ImageMagick (X11), spectacle (KDE) and gnome-screenshot (GNOME). The first installed tool that works wins, region screenshots are cropped for tools that can't, and the `screenshot` config key takes your own command.
- **Screen size detection** for niri, Hyprland (`hyprctl`), sway (`swaymsg`), X11 (`xrandr`) and wlr-randr, with rotated outputs handled. The last resort is the size of a screenshot, so it still works on compositors nobody planned for.
- **A dialog gate** for X11, GNOME and anything without layer-shell: a zenity, yad or kdialog card with Allow, Allow for N min and Deny, silence is a no, plus a notification while the pointer is in use. `ghosthand revoke` still works with no overlay, and `mode`, `countdown` and `grant_minutes` apply the same way. The `gate` config key chooses `auto` (default), `overlay` or `dialog`.
- **Agent skill** at `skills/ghosthand/SKILL.md` (valid frontmatter, validated in the tests) that teaches an agent the whole thing from the shell, no MCP needed. `ghosthand skill` prints it and `ghosthand skill install [DIR]` installs it.
- `ghosthand doctor` now reports the session, the keyboard layout, the clipboard tool, the screenshot tool, the screen-size source and what will ask you.
- A new pixel-art mascot: a ghost with a cursor on its chest (`assets/src/make_logo.py` draws it).
- Docs for multiple monitors, Hyprland and sway device mapping, and X11.

### Changed

- **MCP server updated to spec 2026-07-28**: stateless requests with `_meta` protocol version, `server/discover`, `resultType`, `ttlMs` and `cacheScope` on results, the `-32022` unsupported-version error, and server info in `_meta`. Clients on 2025-11-25 and older (`initialize`) keep working.
- The MCP `pointer` tool takes the keyboard actions too (it is still called `pointer`), and the skill covers them.
- MCP tools now have titles, annotations (`readOnlyHint`, `destructiveHint` and the rest), output schemas and `structuredContent`. An unknown tool is a protocol error (`-32602`) and bad arguments are tool errors the agent can fix.
- The logo and banner were redrawn.
- A clearer message when nothing is there to ask you, and `ghosthand status` and `revoke` work with either gate.
- Python 3.9 compatible (CI runs 3.9, 3.11 and 3.13).

### Notes

- Only niri has been run on a real desktop. The other compositors are covered by tests against sample output of each tool.

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

[0.2.0]: https://github.com/CLOUDWERX-DEV/Ghosthand/releases/tag/v0.2.0
[0.1.0]: https://github.com/CLOUDWERX-DEV/Ghosthand/releases/tag/v0.1.0
