# Changelog

All notable changes to Ghosthand are in this file. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project uses [Semantic Versioning](https://semver.org/).

## [0.6.0] - 2026-11-27

### Added

- **A theme file for the overlay.** `~/.config/ghosthand/theme.json` (all keys optional: `surface`, `raised`, `text`, `muted`, `text_on_accent`, `danger`, `warn`, `hairline`, `accent`) sets the colours of the card, the session pill, the ring and the tag, and the overlay follows the file live. Without the file nothing changes; a missing or broken file is ignored; `accent` in `config.json` still wins. Anything that can write JSON (a desktop theme switcher, a script) can keep Ghosthand in step with the rest of the desktop. See `docs/configuration.md`.

## [0.5.1] - 2026-11-27

### Changed

- **A new logo**: the angry ghost with a cursor on its chest. It replaces the pixel ghost in the icon, the banner and the card in the screenshot, and the overlay's card now shows it too (drawn as a path, so there is still no image or icon font). The mark is `assets/src/logo.svg`, traced from the artwork in `assets/src/logo-master.png`, and `cargo run --example make_logo` builds the icon and the banner around it.

## [0.5.0] - 2026-11-27

Agents get the "ask once" rule before their first call, and the little annoyances around upgrades and sessions are gone.

### Added

- **`ghosthand rules [install|remove|status] [FILE...]`**: a short, marked block of rules for an agent's global instructions file (the one it reads on every start: Claude Code, Codex, Antigravity, opencode). It tells an agent to ask for one session instead of a card per call and never to go around the gate. Installing again updates it in place, removing it leaves the rest of the file as it was, it never creates a file, and `--dry-run` shows what would change. See `docs/agent-rules.md`.
- **`ghosthand skill install --all`** installs the skill into the skills folder of every agent that is installed.
- **`session start` on a session that is already on** and covers the request returns `already_on` and the time left, with no second card. An agent can call it at the top of every task.
- **A hint when a session is too narrow** (typing inside a mouse session): the reply says to start a new one with `--scope both`.
- The pill shows who is driving, turns amber for the last minute, and can sit at the top (`"pill_position": "top"` in the config).
- The MCP server's instructions carry the "ask once" rule, so MCP clients get it without installing anything.
- `ghosthand doctor` flags an out of date overlay and lists the agent rules in each global agent file.

### Fixed

- **A stale overlay after an upgrade.** `ghosthand overlay` now notices when the installed overlay differs from the one in the binary, keeps the old files in `<name>.previous` and installs the new ones. A symlink to a checkout is left alone. `setup` writes a `VERSION` file next to the overlay.

## [0.4.3] - 2026-11-27

### Fixed

- The first session pill after the overlay starts could come up too narrow, with the time and End button cut off. The pill's window is now a fixed width with the pill centred in it, and a mask hands every click outside the pill back to the desktop.

## [0.4.2] - 2026-11-27

Session pill fixes, and fewer cards.

### Fixed

- **The pill is always on top.** Surfaces in the overlay layer stack in the order they were mapped, so a launcher, drop-down terminal or shell panel mapped after the pill sat over it (and took its clicks). The pill is now its own small window; two copies take turns and the spare is mapped above everything every two seconds, then the old one goes. A pill the pointer is on is never swapped, so a click on End never loses its window.
- **End now ends it.** The overlay kept the last session it had read after the file was deleted, so the pill stayed up after End, `session end`, a revoke or the time running out. A missing file now means no session, and End hides the pill at once.
- **The End button fits the pill:** equal padding around it, rounded to match, and the clock reserves its width so nothing shifts as it counts down.

### Added

- **A nudge toward sessions.** When the person has had to Allow two or more separate runs inside ten minutes with no session on, the run still works and prints a hint (stderr; `hint` in the MCP result) telling the caller to start one session instead of a card per call. The MCP `pointer` description and the agent skill now say it up front: more than one call means start a session first.

### Changed

- The full-screen overlay window is only up for the card and the cursor tag, not for the whole session.

## [0.4.1] - 2026-11-27

### Changed

- The note to autonomous agents in `CONTRIBUTING.md` is part of the tagged release. No code changed.

## [0.4.0] - 2026-11-27

Sessions: one approval for a stretch of work.

### Added

- **`ghosthand session start|end|status`**: ask once for a long job (UI testing, say) and runs that fit what was approved skip the card until it ends. Options: `--reason` (required, shown on the card), `--for` (default 30m, up to 8h), `--idle` (default 10m), `--scope both|mouse|keyboard` and `--who`.
- **A pill on screen** while a session is on, with the time left and an End button, in the overlay. The dialog fallback shows the same card (without "Allow for N min") and a notification instead of a pill.
- A session ends on its time, after the idle time with no use, on End, `session end` or a revoke, and when the screen locks. It never covers more than was approved: a mouse session still asks before anything types. It's refused on a locked screen, and with nothing to ask with it doesn't start.
- **A `session` MCP tool** (`start`, `status`, `end`). An agent can ask for a session, never approve one.
- The agent skill and docs (`docs/sessions.md`) explain when to ask for a session and how it ends.

### Changed

- The overlay window now stays up for as long as a session lasts, so the pill is always visible.

## [0.3.1] - 2026-11-26

Cleanup, a fuller contributing guide and more tests.

### Added

- **Notification actions as a last-resort way to ask** (`notify-send` with Allow, Allow for N min and Deny buttons) for desktops with no zenity, yad or kdialog and a notification daemon that shows action buttons. No answer is still a no.
- `examples/live_test.rs`: checks the real devices end to end without anything reaching your desktop (it grabs them exclusively), the second call's speed, and that the daemon and its devices go away when idle.
- Random-input tests for action parsing and MCP messages, so nothing an agent sends can panic.
- A much bigger `CONTRIBUTING.md` (architecture map, recipes for adding a screenshot tool, a desktop or an action, how to test the real devices safely), plus issue and pull request templates, including a desktop report.

### Fixed

- `ghosthand status` and the MCP `gate_status` tool said "unreachable" when no overlay was running but a dialog tool could ask. They now say `idle`.

### Changed

- The logo generator is now a Rust example (`cargo run --example make_logo`).
- The docs say plainly that Quickshell is optional everywhere, and no example refers to any particular desktop setup.
- Release binaries are built on an older Linux so they run on more distributions.

## [0.3.0] - 2026-11-26

Rewritten in Rust, and much faster.

### Changed

- **The whole program was rewritten from scratch in Rust**: one binary of about 1 MB with four direct dependencies (`serde_json`, `libc`, `libloading`, `base64`). There is nothing to install but the binary. libxkbcommon is loaded at run time when it's there (and the built-in US layout is used when it isn't), so building needs no C headers. Every behavior, action, exit code and config key from 0.2.0 is kept.
- Install with `cargo install --git https://github.com/CLOUDWERX-DEV/Ghosthand`, or take a binary from the releases page. The PKGBUILD builds with cargo. The overlay and the agent skill are built into the binary, so `ghosthand setup` and `ghosthand skill install` work from a single file.
- Version numbers come from `Cargo.toml`.

### Added

- **A device daemon** (`ghosthand daemon`, started on demand). A new virtual device takes about a second to be picked up by the compositor, which every call used to wait for. The first call now starts a small background process that keeps the pointer and keyboard alive and forwards events over a Unix socket (mode 0600), so later calls start in milliseconds. It exits after two minutes idle, releases any key or button a dead client was holding, and is off with `"daemon": false`. New config keys: `daemon` and `daemon_idle_seconds`.
- The pointer and keyboard devices are made at the same time, so a first call that needs both waits for one device, not two.
- 110 tests in Rust: unit tests with fake devices, and end-to-end CLI tests that run the real binary in a throwaway directory. CI runs tests, clippy and a release build, and tagged releases build x86_64 and aarch64 binaries.

### Fixed

- **A stray key can no longer allow a request in the dialog gate.** The zenity dialog now focuses Deny by default, and an Allow that comes back less than 0.7 s after the dialog appeared is treated as a no. (A dialog taking focus while you type can otherwise take an Enter meant for something else.)
- Revoke checks between steps are a file check instead of a process start per action, and typing checks for a revoke every 24 characters.
- `ghosthand status` and the MCP `gate_status` tool report the gate that is actually in use (overlay or dialog).

### Performance

Same three calls through both versions, on niri, in a throwaway runtime directory: the first call (jump, click, type) went from 2.56 s to 1.34 s, the second from 2.51 s to 0.26 s, and a bare `pos` from 1.29 s to 0.03 s.

## [0.2.0] - 2026-11-26

Keyboard support, works beyond niri, speaks the current MCP spec, and ships an agent skill.

### Added

- **Keyboard**, through a second virtual device (`ghosthand-keyboard`): `key` (shortcuts and chords like `ctrl+shift+t`, sequences like `Tab Tab Return`), `keydown` and `keyup` (hold Shift while you click), `type` (text, with `\n` and `\t`) and `paste` (clipboard plus Ctrl+V, restoring the clipboard, for text `type` can't make).
- **Your real layout**: `type` and single-character keys are resolved through the layout with libxkbcommon (loaded at run time, so nothing extra to install), so QWERTY, AZERTY, Dvorak, German and the rest work, including Shift and AltGr characters. The layout is detected from the config, the environment, `setxkbmap` or `localectl`, with a built-in US layout as the fallback.
- **`focus`** prints the focused window (niri, Hyprland, sway, X11) so an agent knows where its keys will land. It needs no approval.
- **A gate that knows about the keyboard**: the card says "mouse", "keyboard" or "mouse and keyboard" and shows exactly what will be typed or pressed. A standing "Allow for N min" for the mouse does not cover the keyboard, they're granted separately. This works in the overlay and in the dialog fallback.
- Keyboard config: `keyboard` (false switches it off), `keyboard_layout`, `keyboard_variant`, `keyboard_options`, `type_delay_ms` and `paste_terminal`.
- Held keys and buttons are always released at the end, on error and on revoke, and a revoke stops typing within a couple of dozen characters. Typed text is checked against the layout before anything is pressed.
- **Screenshots on any session**: grim (niri, Hyprland, sway, river), maim, scrot or ImageMagick (X11), spectacle (KDE) and gnome-screenshot (GNOME). The first installed tool that works wins, region screenshots are cropped for tools that can't, and the `screenshot` config key takes your own command.
- **Screen size detection** for niri, Hyprland (`hyprctl`), sway (`swaymsg`), X11 (`xrandr`) and wlr-randr, with rotated outputs handled. The last resort is the size of a screenshot, so it still works on compositors nobody planned for.
- **A dialog gate** for X11, GNOME and anything without layer-shell: a zenity, yad or kdialog card with Allow, Allow for N min and Deny, silence is a no, plus a notification while the pointer is in use. `ghosthand revoke` still works with no overlay, and `mode`, `countdown` and `grant_minutes` apply the same way. The `gate` config key chooses `auto` (default), `overlay` or `dialog`.
- **Agent skill** at `skills/ghosthand/SKILL.md` (valid frontmatter, validated in the tests) that teaches an agent the whole thing from the shell, no MCP needed. `ghosthand skill` prints it and `ghosthand skill install [DIR]` installs it.
- `ghosthand doctor` now reports the session, the keyboard layout, the clipboard tool, the screenshot tool, the screen-size source and what will ask you.
- A new pixel-art mascot: a ghost with a cursor on its chest (`cargo run --example make_logo` draws it).
- Docs for multiple monitors, Hyprland and sway device mapping, and X11.

### Changed

- **MCP server updated to spec 2026-07-28**: stateless requests with `_meta` protocol version, `server/discover`, `resultType`, `ttlMs` and `cacheScope` on results, the `-32022` unsupported-version error, and server info in `_meta`. Clients on 2025-11-25 and older (`initialize`) keep working.
- The MCP `pointer` tool takes the keyboard actions too (it is still called `pointer`), and the skill covers them.
- MCP tools now have titles, annotations (`readOnlyHint`, `destructiveHint` and the rest), output schemas and `structuredContent`. An unknown tool is a protocol error (`-32602`) and bad arguments are tool errors the agent can fix.
- The logo and banner were redrawn.
- A clearer message when nothing is there to ask you, and `ghosthand status` and `revoke` work with either gate.

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
- Unit tests for the gate, actions, session and MCP, run in CI.
- Docs: setup, configuration, how it works, MCP, other compositors, security and contributing.

[0.4.1]: https://github.com/CLOUDWERX-DEV/Ghosthand/releases/tag/v0.4.1
[0.4.0]: https://github.com/CLOUDWERX-DEV/Ghosthand/releases/tag/v0.4.0
[0.3.1]: https://github.com/CLOUDWERX-DEV/Ghosthand/releases/tag/v0.3.1
[0.3.0]: https://github.com/CLOUDWERX-DEV/Ghosthand/releases/tag/v0.3.0
[0.2.0]: https://github.com/CLOUDWERX-DEV/Ghosthand/releases/tag/v0.2.0
[0.1.0]: https://github.com/CLOUDWERX-DEV/Ghosthand/releases/tag/v0.1.0
