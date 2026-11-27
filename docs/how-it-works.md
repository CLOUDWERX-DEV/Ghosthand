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
4. **Act.** After Allow, only the devices the run needs are created on `/dev/uinput`: a virtual absolute pointer (`ghosthand-pointer`) for mouse actions, and a virtual keyboard (`ghosthand-keyboard`) for key actions. Positions are scaled from screenshot pixels to the pointer's range. Characters are turned into key presses through your keyboard layout.
5. **Show.** Each step appends `pos X Y OUTPUT` to `$XDG_RUNTIME_DIR/ghosthand/stream`. The overlay follows that file with `tail -F` and moves the tag and ring.
6. **Stay revocable.** Between every action, and every slice of a long `wait`, the CLI asks the overlay if the run was revoked.
7. **Clean up.** Held buttons and keys are released, the pointer's last position is saved, the stream gets `end`, and the device is closed. This happens on success, error and revoke.

## Files

- `ghosthand/` the Python package: `cli`, `actions`, `session`, `gate`, `dialog`, `uinput`, `keyboard`, `keys`, `layout`, `windows`, `screen`, `geometry`, `config`, `doctor`, `mcp`.
- `ghosthand/overlay/` the Quickshell config: `Gate.qml` (state and the countdown), `Overlay.qml` (card, tag, ring), `Theme.qml`, `Btn.qml`, `PointerGlyph.qml`, `shell.qml` (the IPC handler).
