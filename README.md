<p align="center">
  <img src="assets/banner.png" alt="Ghosthand" width="720">
</p>

<p align="center">
  <a href="CHANGELOG.md"><img alt="version" src="https://img.shields.io/badge/version-0.3.0-3de0ff?style=flat-square"></a>
  <a href="LICENSE"><img alt="license" src="https://img.shields.io/badge/license-MIT-b26bff?style=flat-square"></a>
  <img alt="rust" src="https://img.shields.io/badge/rust-single%20binary-3de0ff?style=flat-square">
  <img alt="platform" src="https://img.shields.io/badge/Wayland-niri%20first-b26bff?style=flat-square">
  <img alt="tests" src="https://github.com/CLOUDWERX-DEV/Ghosthand/actions/workflows/ci.yml/badge.svg">
</p>

<p align="center"><b>Digital Food for the Analog Soul</b><br>by <a href="https://cloudwerxlab.com">CLOUDWERX LAB</a></p>

---

Ghosthand lets a coding agent use your real mouse and keyboard on Linux. Right click a menu, drag something, hover a tooltip, scroll a list, type into a field, hit a shortcut, then take a screenshot and see what happened. It was built for niri on Arch and CachyOS, because that's what I run, and that's where it's been tested the most. It's meant to work on X11, Hyprland, sway, KDE and GNOME too (the table below is honest about what's been verified).

The part I care about most is the gate. Before anything moves or types, a card shows up on your screen saying who's asking, why, and exactly what it's about to type. Nothing happens until you click Allow. No answer counts as a no. While the agent is driving, a little "Agent Control" tag and a ring follow the cursor so you always know it's not you. One key takes the pointer back at any time.

<p align="center">
  <img src="assets/gate.png" alt="The allow/deny card, and the tag that follows the cursor" width="720"><br>
  <sub>An illustration of the card and the tag, not a screenshot of a real desktop.</sub>
</p>

## Why it exists

I got tired of agents that could write a whole UI but couldn't open a right-click menu on it. Screenshots tell you what's there. They can't tell you what happens when you hover or drag. So the agent has to be able to use the pointer, and I had to be able to trust that it only does that when I say so.

How it works under the hood is simple. Ghosthand creates a virtual absolute pointer through `/dev/uinput`, so the display server sees a normal mouse. That's why it isn't tied to one compositor: no portals, no compositor plugins, no X11 libraries. The card and tag are a small Quickshell config (Wayland), or a plain zenity/yad/kdialog dialog where Quickshell can't run. Everything else is one small Rust binary.

## What you get

- **A real pointer.** Click, double click, right click, middle click, drag, right drag, scroll both ways, hover.
- **A real keyboard.** Type text, press shortcuts (`ctrl+shift+t`), hold keys while you click, paste what can't be typed. It reads your actual keyboard layout (QWERTY, AZERTY, Dvorak, German and so on) so it types what it should. [Details](docs/keyboard.md).
- **Knows where keys will land.** `focus` prints the focused window, so an agent can check before it types.
- **A gate.** Allow, Allow for N minutes, or Deny, on a card with a countdown. Silence is a no. The card shows what will be typed, and allowing the mouse never allows the keyboard.
- **A visible tag and ring** that follow the cursor while an agent drives it (with the Quickshell overlay, see below).
- **A panic key.** `ghosthand revoke` stops a running agent, ends any grant and closes the card. Bind it to a key.
- **Screen coordinates that match screenshots.** You give it pixels from a `grim` screenshot and it handles the scale.
- **An agent skill**, so an agent can learn to use it from the shell with no MCP at all. `ghosthand skill install` drops it in place.
- **An MCP server** on the current spec (2026-07-28), and older clients still work. There's no tool to approve a request. That part's yours.
- **A lock check.** It refuses to run while your session is locked, and if it can't tell, it counts as locked.
- **Works where you are.** Screenshots and screen size are detected for niri, Hyprland, sway, X11 and the rest, see below.
- **Fast.** One static-ish Rust binary (about 1 MB, four direct dependencies). Devices are kept alive by a background daemon, so after the first call a run starts in milliseconds instead of waiting a second for a new virtual device. See [the numbers](#how-fast-is-it).

## Where it works

Two parts, and they're independent: the **pointer** (uinput, works on anything) and the **asking** (the card). The card uses Quickshell on Wayland, or a dialog everywhere else.

| Session | Pointer | Screenshots | Screen size | The card | Tested |
| --- | --- | --- | --- | --- | --- |
| **niri** | yes | grim | `niri msg` | Quickshell overlay + tag | **yes, daily** |
| **Hyprland** | yes | grim | `hyprctl` | Quickshell overlay + tag | parsers tested, not run live |
| **sway, river, other wlroots** | yes | grim | `swaymsg` / `wlr-randr` | Quickshell overlay + tag | parsers tested, not run live |
| **KDE Plasma (Wayland)** | yes | spectacle | screenshot size | overlay (layer-shell) or dialog | not tested |
| **GNOME (Wayland)** | yes | gnome-screenshot | screenshot size | dialog (no layer-shell) | not tested |
| **X11** | yes | maim, scrot or ImageMagick | `xrandr` | dialog | parsers tested, not run live |

"Parsers tested" means the code that reads each tool's output is covered by tests with real output samples, but I haven't driven those desktops myself. If you run one of them, `ghosthand doctor` and a bug report or a pull request with what happened would help a lot. On X11 and GNOME the card is a dialog and there's no cursor tag, a notification tells you the pointer's in use instead.

**Multiple monitors:** coordinates are for the output Ghosthand reports (`ghosthand info`). Compositors map a virtual absolute pointer across all outputs by default, so on a multi-monitor setup you'll want to pin it to one. [Notes for Hyprland and sway are in the setup docs.](docs/setup.md#multiple-monitors)

## Install

You need a screenshot tool for your session (`grim` on Wayland, `maim`, `scrot` or ImageMagick on X11). For the card, either [Quickshell](https://quickshell.org) (Wayland with layer-shell) or one of `zenity`, `yad` or `kdialog`.

```sh
# from source (Rust 1.74 or newer)
cargo install --git https://github.com/CLOUDWERX-DEV/Ghosthand

# or grab a binary from the releases page and put it on your PATH

ghosthand setup             # puts the overlay in ~/.config/quickshell/ghosthand (skip it if you'll use a dialog)
```

Then give your user access to `/dev/uinput`. This is the one step that needs root, once:

```sh
sudo cp packaging/99-ghosthand-uinput.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger /dev/uinput
sudo usermod -aG input "$USER"      # log out and back in
```

Start the overlay (skip this on X11/GNOME, the dialog needs nothing running) and check everything:

```sh
ghosthand overlay &         # or autostart it, see docs/setup.md
ghosthand doctor
```

`doctor` tells you which screenshot tool, screen-size source and card it picked. With `"gate": "auto"` (the default) it uses the overlay when it's running and a dialog when it isn't.

On Arch there's a [PKGBUILD](packaging/PKGBUILD) too.

## Use it

Each action is one quoted argument. Coordinates are pixels in a `grim` screenshot.

```sh
ghosthand --who Claude --reason "Checking the bar menu" \
  'jump 800 26' 'wait 0.4' 'rclick' 'wait 0.8' 'shot /tmp/menu.png'
```

| Action | What it does |
| --- | --- |
| `jump X Y` | put the pointer there at once |
| `move X Y` | glide there in small steps, so hover states follow |
| `click` `dclick` `rclick` `mclick` `[X Y]` | click where it is, or at X Y |
| `down` `up` `rdown` `rup` | hold and release a button |
| `drag X1 Y1 X2 Y2` | press, glide, release (`rdrag` uses the right button) |
| `scroll N [X Y]` `hscroll N [X Y]` | wheel notches, positive scrolls the content down |
| `wait SECONDS` | sleep for animations (a revoke still stops it) |
| `shot FILE [X Y W H]` | screenshot of the output or a region |
| `pos` `info` | where the pointer is, and the output's size and scale |
| `focus` | the focused window as JSON, so you know where keys will land |
| `key COMBO...` | press keys and chords in order: `key ctrl+s`, `key Escape`, `key Tab Tab Return` |
| `keydown COMBO` `keyup COMBO` | hold and release keys, for Shift+click and friends |
| `type TEXT` | type text with your real layout (`\n` is Enter, `\t` is Tab) |
| `paste TEXT` | clipboard plus Ctrl+V for text `type` can't make, like emoji. The clipboard is put back |

Typing works the same way. Click the field first, because keys go to whatever has focus:

```sh
ghosthand --who Claude --reason "Searching the launcher for the settings app" \
  'key super+space' 'wait 0.5' 'focus' 'type settings' 'wait 0.4' 'shot /tmp/search.png'
```

Typos, unknown keys, and characters your layout can't type are caught before anything moves or is typed, so a bad action can't leave you half way through a drag or a sentence. If something goes wrong or you revoke, held buttons are released.

Exit codes: `0` done, `1` an action failed or the session is locked, `2` bad usage, `3` taken back, `4` denied or no answer, `5` the overlay can't be reached.

### Teach an agent with the skill (no MCP needed)

There's a proper agent skill in [`skills/ghosthand`](skills/ghosthand/SKILL.md). It covers the look, aim, act, check loop, coordinates, every action, how to behave at the gate, and what the exit codes mean. Install it where your agent looks for skills:

```sh
ghosthand skill install                       # ~/.claude/skills/ghosthand (Claude Code)
ghosthand skill install ~/.agents/skills      # or any skills folder (Codex, shared agents, ...)
```

After that the agent just runs the `ghosthand` command from its shell.

### Or use it over MCP

```json
{
  "mcpServers": {
    "ghosthand": { "command": "ghosthand", "args": ["mcp"] }
  }
}
```

Claude Code: `claude mcp add ghosthand -- ghosthand mcp`

The tools are `pointer`, `screenshot`, `screen_info` and `gate_status`. `pointer` needs a `reason`, because that's what you read on the card. `screenshot` doesn't need approval since it never touches the pointer. The server speaks the current MCP spec (2026-07-28, stateless, with `server/discover`) and still answers `initialize` for clients on 2025-11-25 and older. [More in the docs.](docs/mcp.md)

### Bind the panic key

For niri, add this to your config (the full snippet is in [`packaging/niri-snippet.kdl`](packaging/niri-snippet.kdl)):

```kdl
binds {
    Mod+Ctrl+Alt+Escape allow-inhibiting=false { spawn "ghosthand" "revoke"; }
}
```

## Config

`~/.config/ghosthand/config.json`, read by the overlay, and only by the overlay. A program asking for the pointer can't change any of it.

```json
{
  "mode": "ask",
  "countdown": 8,
  "grant_minutes": 10,
  "accent": "#3de0ff"
}
```

- `mode`: `ask` (must be allowed, default), `notify` (a countdown that goes ahead unless you deny) or `off` (no card, the tag still shows).
- `countdown`: seconds the card waits, 2 to 60. An agent can ask for longer, never shorter.
- `grant_minutes`: what "Allow for N min" grants, 1 to 240.
- `accent`, `ui_scale`: the look (overlay).
- `gate`: `auto` (default), `overlay` or `dialog`.
- `screenshot`: a command that writes a PNG to `{file}`, if you want to use your own tool.

More in [docs/configuration.md](docs/configuration.md).

## How it compares

There are other tools doing nearby things, and some are very good. I'm not going to claim I beat them at anything, because I haven't benchmarked it. Here's the honest version of how they differ:

- [**Cua Drivers**](https://github.com/trycua/cua) is a big, polished project that covers sandboxes, VMs and more across platforms. If you want that scope, use it. Ghosthand is much smaller and only does the local pointer on Wayland.
- [**computer-use-linux**](https://github.com/agent-sh/computer-use-linux), [**Deskwright**](https://github.com/tristanmuzzu/deskwright) and **hypruse** also give agents a desktop to drive. If you're on Hyprland or want accessibility trees or window control, look at those first.

What Ghosthand focuses on is the human side of it: a gate you answer, a tag you can't miss, and a key that takes everything back. And it's built around niri on Arch/CachyOS, where I got tired of nothing quite fitting. It's also small: you can read the whole thing in an afternoon.

## Limits, said plainly

- **It isn't a security boundary.** It's a courtesy gate. An agent with a shell and write access to `/dev/uinput` can skip Ghosthand and make its own pointer. The gate protects you from an agent that's behaving, not one that's hostile. If you need a hard boundary, run the agent somewhere that doesn't have `/dev/uinput`. Read [SECURITY.md](SECURITY.md).
- **Typing into a terminal runs commands.** That's the point of the card showing you exactly what will be typed. Read it before you click Allow.
- **Keys go to the focused window.** Ghosthand can't aim them. `focus` tells the agent where they'll land, and it's on the agent to click the field first.
- **The tag needs the Quickshell overlay.** The dialog fallback asks you the same question but can't draw a ring or tag. GNOME has no layer-shell, so it's dialog only there.
- **Only niri has been run for real.** The others are built from each tool's documented output and tested against samples. See the table above.
- **Absolute pointer on several monitors** needs pinning to one output on some compositors (docs/setup.md).
- **A running `pointer` call can't be cancelled by the MCP client**, since it's a blocking call. Use `ghosthand revoke`.

## How fast is it?

A new virtual input device takes about a second to be picked up by the compositor. The first version paid that on every single call. Now the first call starts `ghosthand daemon` in the background, which keeps the devices alive and forwards events from later calls over a Unix socket (mode 0600). It exits by itself after two minutes of nobody using it, and lets go of any key or button a dead client was holding. The gate is not in the daemon, it stays in every run.

Revoke checks used to start a process between every action. They're a file check now.

Measured on my machine (niri, one pointer and keyboard, the same three calls through both versions, in a throwaway runtime directory):

| Call | Python version | Rust version |
| --- | --- | --- |
| first call: jump, click, type | 2.56 s | 1.34 s (the devices are made once, together) |
| second call, a few seconds later | 2.51 s | 0.26 s |
| `pos` | 1.29 s | 0.03 s |

Turn the daemon off with `"daemon": false` in the config if you'd rather every run made its own devices (the old behavior). `daemon_idle_seconds` changes how long it lingers, `0` keeps it running.

## Develop

```sh
cargo test
cargo clippy --all-targets
```

The unit tests use fake devices and the CLI tests run the real binary in a throwaway directory, so nothing touches your desktop and uinput isn't needed. See [CONTRIBUTING.md](CONTRIBUTING.md). The logo is drawn by `assets/src/make_logo.py`.

## Support

If it's useful to you, you can [buy me a coffee](https://buymeacoffee.com/cloudwerxl3). More from us at [cloudwerxlab.com](https://cloudwerxlab.com) and [github.com/CLOUDWERX-DEV](https://github.com/CLOUDWERX-DEV).

MIT licensed. See [LICENSE](LICENSE).
