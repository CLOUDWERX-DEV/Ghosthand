# How it works

```
agent ──► ghosthand CLI / MCP ──► ask ──► overlay (card) ──► you: Allow
                │                                   │
                │ allowed                           │ tag + ring follow
                ▼                                   ▼
        /dev/uinput pointer ──► compositor      stream file
                │
                └─ every step checks "revoked?"
```

1. **Validate.** Every action is parsed first. A typo exits 2 before anything happens.
2. **Check the lock.** A locked session, or one that can't be checked, is refused.
3. **Ask.** Unless the run is only `shot`, `wait`, `info` and `focus`, the CLI asks the overlay over Quickshell IPC (`qs -c ghosthand ipc call ghosthand ask ...`) and polls `status`. The card shows the request: who, why, what it takes (mouse, keyboard, or both) and exactly what will be typed.
4. **Act.** After Allow, only the devices the run needs are used: they come from the device daemon (below), or are created on `/dev/uinput` if it's off. Pointer and keyboard are made at the same time. The devices are: a virtual absolute pointer (`ghosthand-pointer`) for mouse actions, and a virtual keyboard (`ghosthand-keyboard`) for key actions. Positions are scaled from screenshot pixels to the pointer's range. Characters are turned into key presses through your keyboard layout.
5. **Show.** Each step appends `pos X Y OUTPUT` to `$XDG_RUNTIME_DIR/ghosthand/stream`. The overlay follows that file with `tail -F` and moves the tag and ring.
6. **Stay revocable.** Between every action, and every slice of a long `wait`, the CLI asks the overlay if the run was revoked.
7. **Clean up.** Held buttons and keys are released, the pointer's last position is saved, the stream gets `end`, and the device is closed. This happens on success, error and revoke.

## Files

- `src/` the Rust crate: `cli`, `actions`, `session`, `gate`, `dialog`, `uinput`, `keyboard`, `keys`, `layout`, `windows`, `screen`, `geometry`, `config`, `doctor`, `mcp`.
- `ghosthand/overlay/` the Quickshell config: `Gate.qml` (state and the countdown), `Overlay.qml` (card, tag, ring), `Theme.qml`, `Btn.qml`, `PointerGlyph.qml`, `shell.qml` (the IPC handler).

## Sessions

`session start` asks through the gate like any other request, with a card that says it is a session, and on Allow writes `session.json`. Before every gated run, `execute` reads it: if it's active (not expired, not idle too long) and covers the scope, the card is skipped. A revoke deletes it, a locked screen ends it, each run that uses it restarts the idle clock. The overlay watches the file to show the pill. See [sessions.md](sessions.md).

## The device daemon

A new uinput device takes about a second to be noticed by the compositor. So the first run starts `ghosthand daemon` (a new session, no terminal), which creates the pointer and keyboard once and keeps them. Later runs connect to `$XDG_RUNTIME_DIR/ghosthand/devices.sock` (mode 0600, in a 0700 directory) and send raw events as text lines: `open p`, `e p <type> <code> <value>...`, `bye`. If a client disappears holding a key or a button, the daemon releases it. After `daemon_idle_seconds` (120) with nobody connected it exits and its devices go with it.

The gate is not in the daemon. It forwards events and nothing else, so every run still asks first. That also means any process of yours that can reach the socket could inject input without asking, which is the same reach it would have with access to `/dev/uinput`. If that bothers you, `"daemon": false`. See [SECURITY.md](../SECURITY.md).

## Why a revoke is a file

`ghosthand revoke` writes the time to `$XDG_RUNTIME_DIR/ghosthand/revoked` (and tells the overlay to close its card and end any grant). Every run reads that file between steps, and every 24 typed characters. A file check costs microseconds, where asking the overlay over IPC cost a process start each time.
