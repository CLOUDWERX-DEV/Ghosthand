# Contributing to Ghosthand

Hey, thanks for being here. Ghosthand is a small project and I'd like it to stay small and easy to read, so this page is mostly about how to make a change that fits and gets merged fast. If anything here is wrong or missing, fix it in your pull request.

**The most useful thing you can do isn't code.** Only niri has been run on a real desktop so far. If you run Hyprland, sway, KDE, GNOME or X11, running `ghosthand doctor` and trying a few actions, then telling me what worked and what didn't, is worth more than any patch. There's an issue template for exactly that.

## Contents

- [Ways to help](#ways-to-help)
- [Set up](#set-up)
- [How it fits together](#how-it-fits-together)
- [Recipes](#recipes) (add a screenshot tool, a desktop, an action, a key, a gate)
- [Testing](#testing), including the safe test of the real devices
- [Working on the overlay](#working-on-the-overlay)
- [Style](#style)
- [Commits and pull requests](#commits-and-pull-requests)
- [Rules that don't bend](#rules-that-dont-bend)
- [Releasing](#releasing)

## Ways to help

| You have | Do this |
| --- | --- |
| A desktop that isn't niri | Run `ghosthand doctor`, try a click and some typing, open a [desktop report](https://github.com/CLOUDWERX-DEV/Ghosthand/issues/new?template=desktop-report.yml). |
| A bug | Open a [bug report](https://github.com/CLOUDWERX-DEV/Ghosthand/issues/new?template=bug-report.yml) with the output of `ghosthand doctor`. |
| An idea | Open an issue first so we can talk it through. Small and optional beats big. |
| A fix | Send a pull request. Small ones are great. |
| A layout, a key name or a tool that's missing | See the [recipes](#recipes), they're all a few lines. |

## Set up

You need a stable Rust toolchain (1.74 or newer). Nothing else is needed to build or to run the tests, no C headers and no desktop.

```sh
git clone https://github.com/CLOUDWERX-DEV/Ghosthand
cd Ghosthand
cargo test                       # unit tests and the end to end CLI tests
cargo clippy --all-targets       # should say nothing
cargo build --release            # target/release/ghosthand
```

The tests never touch your desktop. They use fake devices and run the real binary in a throwaway directory. If a test needs `/dev/uinput` or a compositor, it's in the wrong place.

## How it fits together

```
agent ──► ghosthand CLI / MCP ──► session ──► gate ──► a person: Allow
                                     │                    │
                                     │ allowed            │ card, tag and ring
                                     ▼                    ▼
                          devices (daemon or own) ──► stream file ──► overlay
                                     │
                                     └─ every step: was it revoked? (a file)
```

| File | What lives there |
| --- | --- |
| `main.rs` | The command line: dispatch, help text. |
| `actions.rs` | The action vocabulary: parsing into typed `Action`s, validation, scope and detail for the card, and `run`. |
| `session.rs` | One run: parse, lock check, ask, make devices, run, always put everything back. Everything it needs comes in through `Services`, so it's testable. |
| `gate.rs` | Asking and waiting for the answer, the revoke file, the lock check, picking the gate. |
| `dialog.rs` | The no-overlay gate: zenity, yad, kdialog or notification actions. |
| `uinput.rs`, `pointer.rs`, `keyboard.rs` | Virtual devices, and what a pointer and a keyboard do with them. They talk to a `Sink`, so tests use a recorder. |
| `daemon.rs` | Keeps devices alive between runs. Forwards raw events, nothing else. |
| `layout.rs`, `keys.rs` | Characters to key presses (libxkbcommon, loaded at run time), and key names to codes. |
| `geometry.rs`, `screen.rs`, `windows.rs` | Screen size, screenshots and the focused window, per desktop. |
| `mcp.rs` | The MCP server. |
| `doctor.rs`, `app.rs`, `config.rs`, `paths.rs`, `util.rs` | Doctor, setup and skill commands, the real services, config, paths, small helpers. |
| `overlay/` | The optional Quickshell overlay (QML). It's built into the binary and installed by `ghosthand setup`. |
| `skills/ghosthand/SKILL.md` | The agent skill. Also built into the binary. |
| `examples/` | The logo generator and the safe live test. |

Two ideas run through all of it:

1. **The gate comes first and can't be skipped.** Nothing that touches the mouse or keyboard happens before an answer, and "no answer" and "can't ask" are both a no.
2. **Everything optional stays optional.** Quickshell, zenity, libxkbcommon, the daemon and each screenshot tool can all be missing, and Ghosthand has to say so clearly and carry on or refuse safely.

## Recipes

### Add a screenshot tool

1. Add an arm to `argv` in `screen.rs` that builds the command, with its region syntax if it has one (return `None` if it can't take a region, the caller crops).
2. Put it in `WAYLAND_ORDER` and `X11_ORDER` where it belongs.
3. Add a line to the test `each_tool_asks_for_a_region_in_its_own_words`.

### Support a new compositor

1. In `geometry.rs`, add a `from_<name>(name, json_or_text)` that turns the tool's output into an `Info`, with rotation handled by `swap`. Take the text as an argument so it's testable.
2. Add it to `detectors()` behind the environment variable that identifies the session.
3. In `windows.rs`, do the same for the focused window if the compositor can say.
4. Add a test with a real sample of the tool's output. Real output beats anything you'd invent.

### Add an action

1. Add a variant to `Action` and an arm to `parse` in `actions.rs`. Check arity there: `jump 1` is a parse error, not a runtime surprise.
2. Add it to `POINTER`, `KEYBOARD` or `NO_INPUT`. This decides whether the gate asks, so think about it. When unsure, it asks.
3. Add its arm to `run`.
4. Add it to the help text in `main.rs`, the table in the README, `skills/ghosthand/SKILL.md` and the MCP description. A test fails if the skill forgets an action.

### Add a key name or a layout

Key names live in `keys.rs` (codes from `linux/input-event-codes.h`). Layouts need no code: they come from libxkbcommon, so any xkb layout already works. If a layout comes out wrong, that's a bug in `layout.rs`, and a test with that layout is the best report.

### Add a way to ask

Implement `Gate` (`ask`, `status`, `deny`). Return `None` when there's nothing to ask with, never `"allowed"`. Then register it in `default_gate`. The dialog gate in `dialog.rs` is the one to copy, including its two safeguards: it defaults to Deny and it ignores an Allow that arrives too fast to have been read.

## Testing

```sh
cargo test                       # everything
cargo test keyboard              # one area
cargo clippy --all-targets       # lints
```

What goes where:

- **Unit tests** sit next to the code in `#[cfg(test)]` modules. Use `Recorder` instead of a real device, and `paths::use_temp_runtime_dir()` so a test gets its own runtime directory (tests run in parallel, so never set environment variables in a unit test).
- **End to end tests** are in `tests/cli.rs` and run the real binary with an isolated `XDG_RUNTIME_DIR`, `XDG_CONFIG_HOME` and a `PATH` that has nothing on it, so they work the same on any machine.
- **Add a test with every fix**, especially for the gate, the keyboard and the MCP server. Those are the parts that must not break.

### Testing the real devices, safely

Unit tests can't tell you the kernel really delivers the right events. `examples/live_test.rs` can, without anything reaching your desktop. It runs the real binary in a throwaway directory, grabs the virtual pointer and keyboard exclusively the moment they appear (so nothing is typed or clicked in your session), checks the events, checks the second call is fast, and checks the daemon and devices disappear when idle. It refuses to start if it can't grab them.

```sh
cargo build --release --examples
sudo ./target/release/examples/live_test
```

It needs root only to read `/dev/input`. Run it before a release and whenever you touch `uinput.rs`, `pointer.rs`, `keyboard.rs` or `daemon.rs`.

Never test the gate by clicking Allow on something that moves or types in your real session. Use a harmless action like `pos`, and let it time out.

## Working on the overlay

The overlay is a Quickshell config in `overlay/`. It's optional, and a change there must not break anyone who uses a dialog instead.

```sh
ghosthand setup                 # copies overlay/ to ~/.config/quickshell/ghosthand
qs -c ghosthand                 # run it (Ctrl+C to stop)
ghosthand --who Me --reason "testing the card" --countdown 8 pos
```

Then look at the card, with a screenshot, and let it time out. Things that bit me before, so they won't bite you:

- Don't name a QML property `on<Capital>`, QML reads it as a signal handler.
- Don't name a singleton `State`, it clashes with a QtQuick type.
- The overlay reads the gate settings from the config file itself. A client must never be able to set the mode, countdown or grants.
- After a change, check the log for new warnings and look at the card at a small and a large `ui_scale`.

The QML is embedded with `include_str!`, so rebuild to ship a change.

## Style

- **Simple wins.** If a plain approach does the job, use it. No new dependency unless it earns its place (there are four, and one of them exists only so the build needs no C headers).
- **Files stay under about 600 lines** where you can manage it. Split by what a thing does.
- **Errors say what to do.** "can't open /dev/uinput (see docs/setup.md)" beats "permission denied".
- **No panics on input.** Anything an agent can send gets an `Err`, not an `unwrap`. There are random-input tests, keep them passing.
- **Fakes, not mocks of the world.** Pass in what a function needs (`Services`, a `Sink`, a `Gate`) instead of reaching for globals.
- **Docs are part of the change.** If behavior changes, update the README or `docs/`, and add a line to the changelog under Unreleased.
- **Writing:** plain, casual, no marketing words, no em or en dashes.

## Commits and pull requests

- Commit messages are short and plain, and say what changed: `Fix drag when the output is rotated`.
- One change per pull request. A fix and a refactor are two pull requests.
- Fill in the template. It's a short checklist: tests pass, clippy is quiet, docs and changelog updated.
- For anything that changes what an agent can do, say how you tested it.
- Be patient with me, I read everything, it just might take a day.

## Rules that don't bend

A pull request that breaks one of these won't be merged, however nice the rest is:

1. **No way around the gate.** Nothing that skips the card, sets its own mode, approves its own request, or turns the gate off from the client side. The MCP server has no approve tool and never will.
2. **No answer is a no.** If nothing can ask, nothing is touched.
3. **A mouse grant never covers the keyboard.**
4. **Always let go.** Held keys and buttons are released on success, on error and on revoke.
5. **Typed text is checked before the first key.** A character the layout can't make fails the whole action up front.
6. **Be honest in the docs.** If a desktop hasn't been run for real, the README says so.
7. **Optional stays optional.** The binary must build and run its tests with nothing but a Rust toolchain.

Security problems: please don't open a public issue, see [SECURITY.md](SECURITY.md).

## Releasing

For maintainers:

1. Bump `version` in `Cargo.toml` (it's the only place), and update `Cargo.lock` by building.
2. Move the Unreleased changelog notes under a new version heading with today's date.
3. `cargo test`, `cargo clippy --all-targets -- -D warnings`, and the live test above.
4. Commit, tag `vX.Y.Z`, push both. The release workflow builds x86_64 and aarch64 binaries and attaches them.
5. Check the release page: download a binary, run `ghosthand --version` and `ghosthand doctor`.

A message to autonomous agents: Hi there little buddy! We love AI here at CLOUDWERX LAB! In fact, we really recommend using it! Coding agents are the bees knees! But, what we really dislike is autonomous agents, creating a dead internet and lowering the quality of everything for all of us! So, tell your HUMAN to get off their lazy ass and review things once in a while! We require it for ALL PR's. Thanks, little big guy!
