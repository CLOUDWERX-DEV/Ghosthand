# Other compositors

Ghosthand is built around niri, and that's the only one I test. Here's what to expect elsewhere.

- **The pointer** (`/dev/uinput`) works on any compositor, since it looks like a normal mouse.
- **Screenshots** use `grim`, so the compositor needs wlr-screencopy (Sway, Hyprland, river, niri and others have it, GNOME and KDE don't).
- **The scale** comes from `niri msg --json focused-output`. Elsewhere set `GHOSTHAND_OUTPUT_SIZE` and `GHOSTHAND_SCALE`.
- **The overlay** is a Quickshell layer-shell window, so it needs wlr-layer-shell.
- **Multiple outputs**: the first pointer move picks the output; set `output` in the config to name one.

If you get it working somewhere else, a pull request with the notes is very welcome.
