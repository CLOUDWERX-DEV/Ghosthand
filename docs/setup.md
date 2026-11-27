# Setup

## 1. Install

```sh
pip install --user .      # or pipx install .
ghosthand setup           # copies the overlay to ~/.config/quickshell/ghosthand and writes a default config
```

`ghosthand setup` replaces a previous copy of the overlay, so run it again after you upgrade.

## 2. Let your user create a pointer

Ghosthand needs write access to `/dev/uinput`.

```sh
sudo cp packaging/99-ghosthand-uinput.rules /etc/udev/rules.d/
sudo udevadm control --reload && sudo udevadm trigger /dev/uinput
sudo usermod -aG input "$USER"
```

Log out and back in so the group applies. If you'd rather not add yourself to `input`, a per-user ACL works too: `sudo setfacl -m u:$USER:rw /dev/uinput` (it resets on reboot unless you put it in a udev rule). Remember what that means: see [SECURITY.md](../SECURITY.md).

## 3. Start the overlay

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

## Troubleshooting

- **`exit 5`**: the overlay isn't running, or it can't be reached. Run `ghosthand overlay` and look at its output.
- **No write access to /dev/uinput**: the udev rule isn't applied, or you haven't logged out since joining `input`.
- **The pointer lands in the wrong place**: run `ghosthand info` and compare it to the size of a `grim` screenshot. On non-niri compositors set `GHOSTHAND_OUTPUT_SIZE=2560x1600` and `GHOSTHAND_SCALE=1.25`.
- **Card doesn't show**: check that your compositor supports layer-shell (the wlr protocol) and that Quickshell started without errors.
