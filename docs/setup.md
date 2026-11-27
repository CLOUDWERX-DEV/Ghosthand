# Setup

## 1. Install

```sh
cargo install --git https://github.com/CLOUDWERX-DEV/Ghosthand   # or: cargo build --release, then copy target/release/ghosthand
ghosthand setup           # copies the overlay to ~/.config/quickshell/ghosthand and writes a default config
```

`ghosthand setup` replaces a previous copy of the overlay, so run it again after you upgrade.

## 2. Let your user create a pointer and keyboard

Ghosthand needs write access to `/dev/uinput`. The same access covers the virtual pointer and the virtual keyboard.

```sh
sudo cp packaging/99-ghosthand-uinput.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger /dev/uinput
sudo usermod -aG input "$USER"
```

Log out and back in so the group applies. If you'd rather not add yourself to `input`, a per-user ACL works too: `sudo setfacl -m u:$USER:rw /dev/uinput` (it resets on reboot unless you put it in a udev rule). Remember what that means: see [SECURITY.md](../SECURITY.md).

## 3. Start the overlay (or use a dialog)

On **X11, GNOME, or anywhere without Quickshell**, skip the overlay. Install `zenity` (or `yad` or `kdialog`) and Ghosthand asks with a dialog instead. It's the same question with the same answers, and a notification tells you the pointer is in use. There's no cursor tag there. `ghosthand doctor` shows which one it will use. Force one with `"gate": "overlay"` or `"gate": "dialog"` in the config.

On Wayland with layer-shell (niri, Hyprland, sway, river, KDE) the overlay gives you the card and the tag.

Run it once by hand to try it:

```sh
ghosthand overlay
```

Then keep it running. Pick one:

- **niri**: `spawn-at-startup "ghosthand" "overlay"` in your config.
- **systemd user unit**: `cp packaging/ghosthand-overlay.service ~/.config/systemd/user/ && systemctl --user enable --now ghosthand-overlay`

## 4. Bind the panic key

```kdl
binds {
    Mod+Ctrl+Alt+Escape allow-inhibiting=false { spawn "ghosthand" "revoke"; }
}
```

`allow-inhibiting=false` keeps it working even when something is grabbing your shortcuts.

## 5. Check

```sh
ghosthand doctor
```

Every line should say `ok`. Then try it:

```sh
ghosthand --who Me --reason "Just testing" 'info' 'pos'
ghosthand --who Me --reason "Just testing" 'jump 400 400'
```

The second one should pop up the card. Allow it and the pointer jumps.

## Multiple monitors

The pointer is one absolute device, and compositors may stretch it over all your outputs. Coordinates are for the output `ghosthand info` reports, so pin the device to it.

- **Hyprland** (`hyprland.conf`): `device { name = ghosthand-pointer  output = eDP-1 }`
- **sway**: `input "4660:22136:ghosthand-pointer" map_to_output eDP-1`
- **X11**: coordinates are for the whole desktop (all monitors together), the same as the screenshot tool captures. Nothing to configure.
- **niri**: maps an absolute device to the first output, which is what you want for one monitor. Set `output` in the config to name another.

## Keyboard notes

- Keys go to the focused window, so an agent has to click the field first. `ghosthand focus` shows what has focus.
- **Layout**: `ghosthand doctor` shows the layout it detected. If typing comes out wrong (a `z` where a `y` should be), set it: `"keyboard_layout": "de"` (and `keyboard_variant` if you use one) in the config. Dead keys and compose sequences aren't typed, use `paste` for those characters.
- **Clipboard** for `paste`: `wl-clipboard` on Wayland, `xclip` or `xsel` on X11.
- **Fast typing dropped by an app?** Raise `type_delay_ms`.
- **Terminals** paste with Ctrl+Shift+V. Set `"paste_terminal": true` if you mostly paste into one.
- The sway device mapping for the pointer doesn't apply to the keyboard.

## Troubleshooting

- **`exit 5`**: the overlay isn't running, or it can't be reached. Run `ghosthand overlay` and look at its output.
- **No write access to /dev/uinput**: the udev rule isn't applied, or you haven't logged out since joining `input`.
- **The pointer lands in the wrong place**: run `ghosthand info` and compare it to the size of a screenshot. If the size can't be detected, set `GHOSTHAND_OUTPUT_SIZE=2560x1600` and `GHOSTHAND_SCALE=1.25`. On several monitors, see above.
- **Card doesn't show**: check that your compositor supports layer-shell (the wlr protocol) and that Quickshell started without errors. No layer-shell (GNOME)? Use a dialog tool.
- **No screenshot**: install `grim` (Wayland), or `maim`/`scrot`/ImageMagick (X11), or set `"screenshot": "mytool {file}"` in the config.
