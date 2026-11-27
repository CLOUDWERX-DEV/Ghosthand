# Configuration

`~/.config/ghosthand/config.json`. `ghosthand setup` writes a default one. The overlay watches the file, so changes apply right away.

| Key | Default | Read by | What it does |
| --- | --- | --- | --- |
| `mode` | `"ask"` | overlay | `ask`: must be allowed, no answer is a no. `notify`: a countdown that goes ahead unless you deny. `off`: no card, the tag still shows. |
| `countdown` | `8` | overlay | Seconds the card waits, 2 to 60. A program can ask for a longer wait but never a shorter one. |
| `grant_minutes` | `10` | overlay | What "Allow for N min" grants, 1 to 240. |
| `accent` | `"#3de0ff"` | overlay | The accent colour. |
| `ui_scale` | `1.0` | overlay | Scales the card and tag, 0.75 to 2. |
| `overlay` | `"ghosthand"` | CLI | The name of the Quickshell config to run and talk to. |
| `lock_check` | loginctl `LockedHint` | CLI | A shell command that prints `no` when the session is unlocked. Anything else, including no answer or an error, counts as locked. |
| `output` | none | CLI | The output name to use for the scale, if you have several. |

## Environment

- `GHOSTHAND_OUTPUT_SIZE` (`2560x1600`) and `GHOSTHAND_SCALE` (`1.25`): skip niri and use these.
- `XDG_RUNTIME_DIR`: where the stream file and the last position live (`$XDG_RUNTIME_DIR/ghosthand/`).

## Why the gate settings aren't an argument

On purpose. If a program could pass `mode: off` when it asks, the gate would be useless. The overlay reads its own config and nobody else.
