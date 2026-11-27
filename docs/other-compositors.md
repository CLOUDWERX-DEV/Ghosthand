# Other compositors and X11

Ghosthand is built around niri, and that's the only desktop I've run it on for real. Everything else is written from each tool's documented output and covered by tests against samples. Here's what happens on each, and what to try if it doesn't work.

| Session | Screenshots | Screen size | Card |
| --- | --- | --- | --- |
| niri | `grim` | `niri msg --json outputs` | Quickshell overlay |
| Hyprland | `grim` | `hyprctl -j monitors` (focused one) | Quickshell overlay |
| sway | `grim` | `swaymsg -t get_outputs -r` | Quickshell overlay |
| river and other wlroots | `grim` | `wlr-randr --json` | Quickshell overlay |
| KDE Plasma | `spectacle` | screenshot size | Quickshell overlay (has layer-shell) or dialog |
| GNOME | `gnome-screenshot` | screenshot size | dialog (no layer-shell) |
| X11 | `maim`, `scrot`, ImageMagick `import` | `xrandr` (whole desktop) | dialog |

- **The pointer** (`/dev/uinput`) works on any of them. It's a normal mouse to the display server.
- **If a screenshot tool fails**, the next installed one is tried, and the error says what each one said. `spectacle` and `gnome-screenshot` can't take a region, so the full screen is cropped with ImageMagick or Pillow when you ask for one.
- **If the size can't be detected**, Ghosthand falls back to the size of a screenshot. Set `GHOSTHAND_OUTPUT_SIZE` and `GHOSTHAND_SCALE` to skip detection.
- **The scale** only matters for placing the cursor tag. Pointer positions don't depend on it.
- **The overlay on X11** isn't supported: Quickshell's click-through overlay relies on Wayland layer-shell. Use the dialog.

If you get a desktop working (or not), please open an issue or a pull request with the output of `ghosthand doctor`.
