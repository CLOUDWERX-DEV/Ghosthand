# Keyboard

Ghosthand has a second virtual device, a keyboard, made the same way as the pointer (`/dev/uinput`). It sends key codes, like any keyboard, so your compositor's own layout and shortcuts apply. Keys go to whatever window has focus.

## Actions

| Action | What it does |
| --- | --- |
| `key COMBO...` | press each key or chord in order, then release it. `key ctrl+s`, `key Escape`, `key Tab Tab Return` |
| `keydown COMBO` | hold keys down (until `keyup`, or the end of the run) |
| `keyup COMBO` | release them |
| `type TEXT` | type text, one key press per character |
| `paste TEXT` | put the text on the clipboard, press Ctrl+V, put the old clipboard back |
| `focus` | print the focused window as JSON, `{"app": "...", "title": "..."}` (or `null`) |

Each action is one quoted argument, so spaces are fine:

```sh
ghosthand --who Claude --reason "Saving the file and running the tests" \
  'key ctrl+s' 'wait 0.3' 'key super+t' 'wait 0.8' 'focus' 'type cargo test\n'
```

## Chords and key names

Join keys with `+`. Modifiers go first: `ctrl+shift+t`, `alt+F4`, `super+Left`, `ctrl+alt+Delete`. Names are case-insensitive.

- **Modifiers**: `ctrl`, `shift`, `alt`, `altgr`, `super` (also `win`, `meta`, `mod`), with `_l` and `_r` forms.
- **Named keys**: `Return`, `Escape`, `Tab`, `BackSpace`, `Delete`, `Insert`, `Home`, `End`, `Page_Up`, `Page_Down`, `Up`, `Down`, `Left`, `Right`, `space`, `Print`, `Pause`, `Menu`, `Caps_Lock`, `F1` to `F24`.
- **Keypad**: `KP_0` to `KP_9`, `KP_Enter`, `KP_Add`, `KP_Subtract`, `KP_Multiply`, `KP_Divide`, `KP_Decimal`.
- **Media**: `XF86AudioPlay`, `XF86AudioNext`, `XF86AudioPrev`, `XF86AudioStop`, `XF86AudioMute`, `XF86AudioRaiseVolume`, `XF86AudioLowerVolume`, `XF86MonBrightnessUp`, `XF86MonBrightnessDown`.
- **Single characters** (`a`, `7`, `/`, `A`, `@`) are the key that makes that character on **your** layout, with Shift or AltGr added for you. So `key ctrl+z` presses the right key on AZERTY and Dvorak too. For the plus key itself, write `ctrl++`.

An unknown key is an error before anything is pressed.

## Typing text

`type` takes everything after the first space, exactly as written, spaces and all. In an action, a newline has to be written as `\n`, a tab as `\t` and a backslash as `\\`:

```sh
ghosthand --reason "Writing a commit message" 'type fix: handle empty config\n'
```

It types with your real layout, pressing Shift and AltGr when a character needs them. Characters your layout has no key for (emoji, or an accent on a US layout) are refused **before anything is typed**, with a hint to use `paste`. Dead keys and compose sequences aren't used.

A revoke stops typing within a couple of dozen characters.

## Pasting

`paste` is for what `type` can't make. It needs `wl-clipboard` (Wayland), `xclip` or `xsel` (X11). It sets the clipboard, presses Ctrl+V (Ctrl+Shift+V if `"paste_terminal": true`), waits a moment for the app to read it, then restores whatever was on the clipboard before.

## Holding keys while you use the mouse

```sh
ghosthand --reason "Selecting a range of files" \
  'move 300 400' 'click' 'move 300 700' 'keydown shift' 'click' 'keyup shift'
```

Anything still held when the run ends (or errors, or you revoke) is released.

## Layouts

Ghosthand reads your layout the way a compositor does, with libxkbcommon (called through ctypes, so there's still nothing to install in Python). The layout is found from, in order: the config (`keyboard_layout`, `keyboard_variant`, `keyboard_options`), the `XKB_DEFAULT_LAYOUT` environment variables, `setxkbmap -query` (X11), `localectl status`, and finally `us`. If libxkbcommon isn't on the system a built-in US layout is used. `ghosthand doctor` shows what it picked.

If you switch layouts on the fly (`us,de`), Ghosthand uses the first one. Set `keyboard_layout` to the one you want.

## The gate and the keyboard

The card says what's being taken ("mouse", "keyboard" or "mouse and keyboard") and shows exactly what will be typed and pressed. A standing "Allow for N min" for the mouse does **not** cover the keyboard: a run that adds typing asks again. `"keyboard": false` in the config switches the whole thing off. More in [SECURITY.md](../SECURITY.md).
