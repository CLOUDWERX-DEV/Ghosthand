# Configuration

`~/.config/ghosthand/config.json`. `ghosthand setup` writes a default one. The overlay watches the file, so changes apply right away.

| Key | Default | Read by | What it does |
| --- | --- | --- | --- |
| `mode` | `"ask"` | overlay | `ask`: must be allowed, no answer is a no. `notify`: a countdown that goes ahead unless you deny. `off`: no card, the tag still shows. |
| `countdown` | `8` | overlay | Seconds the card waits, 2 to 60. A program can ask for a longer wait but never a shorter one. |
| `grant_minutes` | `10` | overlay | What "Allow for N min" grants, 1 to 240. |
| `accent` | `"#3de0ff"` | overlay | The accent colour. |
| `ui_scale` | `1.0` | overlay | Scales the card, the pill and the tag, 0.75 to 2. |
| `pill_position` | `"bottom"` | overlay | Where the session pill sits: `"bottom"` or `"top"`. |
| `gate` | `"auto"` | CLI | `auto`: the overlay if it answers, else a zenity, yad or kdialog dialog. Or force `overlay` or `dialog`. |
| `keyboard` | `true` | CLI | `false` switches every keyboard action off (they're refused up front). |
| `keyboard_layout` | detected | CLI | The xkb layout `type` should assume, like `us`, `de` or `fr`. Only the first of a list is used. |
| `keyboard_variant` | detected | CLI | A variant, like `dvorak` or `nodeadkeys`. |
| `keyboard_options` | none | CLI | xkb options, like `caps:escape`. |
| `type_delay_ms` | `8` | CLI | Milliseconds between typed characters, 0 to 500. Raise it for apps that drop fast input. |
| `paste_terminal` | `false` | CLI | `true` makes `paste` press Ctrl+Shift+V (terminals) instead of Ctrl+V. |
| `daemon` | `true` | CLI | Keep the virtual devices alive in a background process between runs (fast). `false` makes every run create its own. |
| `daemon_idle_seconds` | `120` | CLI | How long the daemon lingers with nobody connected. `0` keeps it running. |
| `screenshot` | none | CLI | A command that writes a PNG to `{file}`, to use your own tool. |
| `overlay` | `"ghosthand"` | CLI | The name of the Quickshell config to run and talk to. |
| `lock_check` | loginctl `LockedHint` | CLI | A shell command that prints `no` when the session is unlocked. Anything else, including no answer or an error, counts as locked. |
| `output` | none | CLI | The output name to use for the scale, if you have several. |

## Environment

- `GHOSTHAND_OUTPUT_SIZE` (`2560x1600`) and `GHOSTHAND_SCALE` (`1.25`): skip niri and use these.
- `XDG_RUNTIME_DIR`: where the stream file and the last position live (`$XDG_RUNTIME_DIR/ghosthand/`).

## Why the gate settings aren't an argument

On purpose. If a program could pass `mode: off` when it asks, the gate would be useless. The overlay reads its own config and nobody else.
