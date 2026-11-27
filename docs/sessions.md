# Sessions

A session is one approval for a stretch of work, so a long job (UI testing, say) doesn't need a card on every call.

```sh
ghosthand session start --reason "UI testing the settings app" --for 30m --idle 10m --scope both
ghosthand session status      # {"active":true,"scope":"mouse and keyboard","seconds_left":1675,"idle_stop_in":590,...}
ghosthand session end
```

| Option | Default | Meaning |
| --- | --- | --- |
| `--reason TEXT` | required | what you read on the card |
| `--for DURATION` | `30m` | how long, from 1 minute to 8 hours: `30m`, `1h`, `1h30m`, `90s`, or a bare number of minutes |
| `--idle DURATION` | `10m` | end it after this long with no use |
| `--scope` | `both` | `both`, `mouse` or `keyboard` |
| `--who NAME` | `Agent` | shown on the card |
| `--countdown N` | the overlay's setting | seconds the card waits |

## What you see

- **One card**, once: who, why, what it covers, and for how long. Just Allow and Deny (the session is the time, so there's no "Allow for N min").
- **A pill** at the bottom of the screen the whole time: "Agent Control · mouse and keyboard · 24:10 left" and an **End** button. It stays above every other window, panel and overlay. The tag and ring still follow the cursor while something is moving it.
- With the dialog fallback there's no pill. You get the same card and a notification when it starts. End it with `ghosthand session end` or your revoke key.

## What it covers, and what ends it

A run inside a session skips the card **only if** the session covers everything it needs. A `mouse` session lets clicks through and still asks before anything types. A session ends when:

- the time is up,
- nothing used it for the idle time,
- you press the pill's End button, run `ghosthand session end`, or `ghosthand revoke`,
- or the session is locked (the next run notices, and refuses).

Starting one is gated like anything else: no answer is a no, and with nothing to ask with it doesn't start (exit 5). It's refused on a locked screen.

## For agents

Ask for a session before the first call of any task that needs more than one. If a plain run is allowed twice inside ten minutes with no session on, the run prints a hint (`hint` in the MCP result) to start one instead of asking again.

Over MCP there's a `session` tool (`start`, `status`, `end`). `start` shows the card and waits for you, and returns an error if you deny or don't answer. There's no way for an agent to approve its own session. The agent skill explains when to ask for one.

## How it works

`session.json` in `$XDG_RUNTIME_DIR/ghosthand/` (mode 0700 directory) holds the scope, the expiry, the idle limit and when it was last used. Every gated run reads it and the clock. The overlay watches the same file to draw the pill, only to display it. Anyone who can write that file can start a session without asking, the same reach they'd have with access to `/dev/uinput`. See [SECURITY.md](../SECURITY.md).
