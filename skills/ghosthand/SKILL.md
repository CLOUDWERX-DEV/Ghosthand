---
name: ghosthand
description: Drive the user's real mouse and keyboard on Linux (Wayland or X11) with the ghosthand CLI to click, right-click, drag, hover, scroll, type, press shortcuts and take screenshots, behind an allow/deny prompt the user answers. Use when a task needs the actual desktop UI rather than code or a browser tool: testing a native or GTK/Qt/Quickshell/Electron app, opening right-click menus, drag and drop, hover states, typing into an app, keyboard shortcuts, checking how a panel really looks, or anything only a person at the desk could do. Works through the shell, so no MCP server is needed.
license: MIT
compatibility: Linux with /dev/uinput access (input group or udev rule). Wayland (niri, Hyprland, sway, river, KDE, GNOME) or X11. Needs the ghosthand CLI (cargo install --git https://github.com/CLOUDWERX-DEV/Ghosthand, or a release binary from github.com/CLOUDWERX-DEV/Ghosthand).
---

# Ghosthand

Ghosthand drives the user's **real** mouse and keyboard through virtual devices (`/dev/uinput`). The compositor sees an ordinary mouse and keyboard, so hover, right click menus, drag and drop, layer-shell panels, tooltips, shortcuts and typing all behave exactly as they do for a person. Everything goes through a gate: the user sees a card ("Claude wants to use your mouse and keyboard", with what you're about to type), nothing happens until they click Allow, and silence counts as a no.

You use it from the shell with the `ghosthand` command. No MCP server is required (there is one, `ghosthand mcp`, if your host prefers tools).

## Ask once: start a session first

**Every card is a click the person has to make. Never make them click Allow call after call.** Before your first input action, decide how many calls the task needs:

- **One call, one short run** (open a menu and look): a plain call with a good `--reason` is fine.
- **Anything else** (testing a panel, walking through a flow, clicking, then checking, then clicking again, a drag and a screenshot, any second or third call): start **one session first** and do all the work inside it.

```sh
ghosthand session start --who Claude --reason "Testing the new tabs panel: open it, drag a tab, check the menus" --for 20m
# ...every call after this runs with no card, until it ends...
ghosthand session end
```

If you have already sent two plain calls, or a reply carries a `hint:` line (stderr; MCP `hint`), that is the signal you got it wrong: start a session now, don't send a third card. Start it with a reason that covers the whole job, a fair `--for`, and `--scope mouse` if you won't type. End it when you're done. Details in **Sessions** below.

## The loop

Always work in this order. Never click blind.

1. **Look.** Take a screenshot and read it.
2. **Aim.** Pick coordinates **from that screenshot** (see Coordinates).
3. **Act.** One `ghosthand` call with a short list of actions and a clear `--reason`. If keys are involved, `focus` first (it says which window will get them).
4. **Check.** Take another screenshot and confirm it did what you meant. If it didn't, look again before trying anything else.

```sh
ghosthand --who Claude --reason "Opening the bar's right-click menu to check it" \
  'info' 'shot /tmp/before.png'
# read /tmp/before.png, find the target, then:
ghosthand --who Claude --reason "Opening the bar's right-click menu to check it" \
  'move 800 26' 'wait 0.4' 'rclick' 'wait 0.8' 'shot /tmp/after.png'
```

Typing works the same way:

```sh
ghosthand --who Claude --reason "Searching the launcher for the settings app" \
  'key super+space' 'wait 0.5' 'focus' 'type settings' 'wait 0.4' 'shot /tmp/search.png'
```

Screenshots, `info`, `focus` and `wait` need no approval. Anything that moves the mouse or presses a key asks.

## Before the first use

Run `ghosthand doctor` once. It checks `/dev/uinput` access, a screenshot tool, the screen size and whether something can ask the user. If it says `NO` somewhere, tell the user what it printed and what it suggests. Do not try to work around a missing piece.

## Coordinates

Coordinates are **pixels of a screenshot** of the output, the same pixels the image has. You never convert for scale; Ghosthand does that.

- Read positions off the screenshot you just took. Don't guess from memory or from a layout you computed.
- `ghosthand info` prints the output's name, size and scale as JSON. If the image you read is a different size than `width` x `height`, you are looking at a resized copy: scale your coordinates up by `width / image_width`.
- To look closely, use a region screenshot: `shot /tmp/z.png X Y W H`. Region coordinates are in the same full-screen pixel space; the image you get back is just that crop.
- `pos` prints where the pointer is right now.

## Actions

Each action is **one quoted argument**. Several in one call run in order.

| Action | What it does |
| --- | --- |
| `jump X Y` | put the pointer there at once |
| `move X Y` | glide there in small steps (use this before a click so hover states follow) |
| `click [X Y]` | left click, here or at X Y |
| `dclick [X Y]` | double click |
| `rclick [X Y]` | right click (context menu) |
| `mclick [X Y]` | middle click |
| `down` / `up` | press / release the left button |
| `rdown` / `rup` | press / release the right button |
| `drag X1 Y1 X2 Y2` | press at 1, glide to 2, release (`rdrag` uses the right button) |
| `scroll N [X Y]` | wheel notches: positive scrolls the content **down**, negative up |
| `hscroll N [X Y]` | sideways wheel |
| `wait SECONDS` | sleep (animations, menus opening). A revoke still interrupts it |
| `shot FILE [X Y W H]` | screenshot of the screen, or of a region |
| `pos` | print the pointer position |
| `info` | print the output's name, size and scale |
| `focus` | print the focused window as JSON (`app`, `title`), or `null` if the desktop can't say |
| `key COMBO...` | press shortcuts and keys in order: `key ctrl+s`, `key Escape`, `key ctrl+shift+t`, `key Tab Tab Return` |
| `keydown COMBO` / `keyup COMBO` | hold or release keys, e.g. `keydown shift`, `click`, `keyup shift` for a Shift+click |
| `type TEXT` | type text, one key press per character, using the user's real layout: `type git status`, `type line one\nline two` |
| `paste TEXT` | put text on the clipboard and press Ctrl+V (then restore the clipboard), for text `type` can't make, like emoji |

A typo in any action, an unknown key, or a character the keyboard layout can't type is caught before anything moves or is typed, so a bad call can't leave a drag half done.

## Sessions (for long jobs)

If you will do more than one call (UI testing, a walkthrough, anything with steps), ask for **one** approval up front instead of one per call. This is the default for a task, not a special case:

```sh
ghosthand session start --who Claude --reason "UI testing the settings app" --for 30m
```

The user sees one card with your reason, what it covers and how long, and a pill stays on their screen (above every other window) with who is driving, the scope, the time left and an End button. The pill turns amber in the last minute. After they allow it, calls inside the scope run without a card. Rules:

- **Give a real reason and a fair duration.** 30 minutes is the default; ask for what the job needs, not the maximum. `--scope mouse` if you won't type.
- **A session never covers more than was approved.** If it was `mouse` only, typing asks again (and the reply carries a hint to start a new one with `--scope both`). Decide the scope up front: if you might type, ask for both.
- **Asking again is free.** `session start` while a session that covers the request is already on returns `already_on` with the time left and shows no card, so you can call it at the top of every task without checking first.
- **Denied or no answer (exit 4): stop and tell the user.** Don't retry.
- **It ends** on the time, after 10 idle minutes, if the user ends or revokes it (exit 3), or when the screen locks. Check with `ghosthand session status`, and end it yourself with `ghosthand session end` when the job is done.
- **Hints.** A reply may end with a line starting `Ghosthand:` after a successful run (over MCP: `hint`). It is advice, not an error: usually "start a session instead of a card per call", or "your session is too narrow". Follow it before the next call.
- Over MCP the same thing is the `session` tool with `action` `start`, `status` or `end`.

## Keyboard

Keys go to whatever window has focus, so **click the field first**, then `focus` to check, then type.

- **Key names** are case-insensitive: `Return`, `Escape`, `Tab`, `BackSpace`, `Delete`, `Home`, `End`, `Page_Up`, `Page_Down`, `Up`, `Down`, `Left`, `Right`, `F1` to `F24`, `space`, `Print`, `Menu`, the keypad (`KP_Enter`, `KP_0`) and media keys (`XF86AudioPlay`, `XF86AudioMute`...). Modifiers are `ctrl`, `shift`, `alt`, `altgr`, `super`. Join a chord with `+`: `ctrl+alt+Delete`. A single character is the key that makes it on the user's layout, so `key ctrl+z` works on QWERTY, AZERTY and Dvorak alike.
- **`type`** takes the text exactly as written after the first space. Write `\n` for Enter, `\t` for Tab and `\\` for a backslash. Shift and AltGr are pressed for you (`Hello`, `@`, `€` on a layout that has it). Characters the layout can't make are refused up front: use `paste` for those.
- **Terminals**: `type` into a terminal runs commands when you end with `\n`. Say what you will run in `--reason`. `paste` uses Ctrl+V; in a terminal the user may have `"paste_terminal": true` set for Ctrl+Shift+V.
- **Chords with the mouse**: `keydown ctrl`, `click 400 300`, `keyup ctrl`. Held keys are always released at the end, even on error or revoke.
- **Never type secrets** (passwords, tokens) unless the user told you to, and don't type into windows you haven't identified with `focus` or a screenshot.
- **Keep the user's review in mind**: the card shows what you will type, so be exact and short. Prefer a few `type` actions you can explain over one giant paste.
- **Pointer grants don't cover the keyboard.** If the user did "Allow for N min" for the mouse, a request that adds typing asks again.

## Technique that works

- **`move` then act.** `move X Y`, `wait 0.3`, then `click`. Many widgets only react after a hover.
- **Wait for UI.** After anything that opens or animates something, `wait 0.5` to `1` before the next action or screenshot.
- **Menus.** `rclick`, `wait 0.8`, `shot`, read the menu, then `move` onto the item and `click`. Menus close if the pointer leaves, so glide, don't jump.
- **Drag and drop.** Use `drag X1 Y1 X2 Y2`, then `wait 0.5` and a screenshot. Some apps need the long form: `move X1 Y1`, `down`, `wait 0.2`, `move XM YM`, `move X2 Y2`, `wait 0.3`, `up`.
- **Scrolling a list.** `scroll 3 X Y` over the list. Take a screenshot after each scroll rather than scrolling a lot at once.
- **Small targets.** Take a region screenshot of the area first and aim at the centre of the target.
- **Check before you type.** `click` the field, `shot` to see the caret, `type`, then `shot` again.
- **Speed.** The first call after a pause takes about a second (a background process makes the virtual devices once). Calls after that start in milliseconds, so several short calls cost almost nothing more than one long one.
- **Keep runs short.** A call of a handful of actions is easier for the user to supervise than a long script.

## The gate: how to behave

The user is watching. A card shows your `--who` and `--reason`, then a tag and ring follow the cursor while you drive.

- **Always set `--who` and a specific `--reason`.** "Checking the Tabs panel after the layout change" is good. "testing" is not.
- **Denied or no answer (exit 4): stop.** Tell the user it was declined and ask what they want. Never retry in a loop, never ask again immediately, never look for a way around the gate. Do not write to `/dev/uinput` yourself, or use `wtype`, `xdotool`, `ydotool` or anything else to move the pointer or press keys to dodge it.
- **Taken back (exit 3):** the user pressed their revoke key. Stop at once and say so. Don't resume unless they ask.
- **Don't make them click through many cards.** Several plain calls in a row is the mistake a session exists to prevent: ask once with `session start`. The user may also Allow for N minutes; later calls then go straight through. Either way keep each run short and purposeful, and end a session when you are done.
- **Never touch anything destructive** (delete, send, buy, close unsaved work, power) without the user having asked for exactly that.
- **The session being locked is a refusal (exit 1).** Don't try to get past a lock screen.

## Exit codes

| Code | Meaning | What to do |
| --- | --- | --- |
| 0 | done | check with a screenshot |
| 1 | an action failed, the keyboard is switched off, or the session is locked | read stderr, fix the action or tell the user |
| 2 | bad usage or an unknown action | fix the call |
| 3 | the user took the pointer back | stop |
| 4 | denied, or no answer in time | stop and ask |
| 5 | nothing to ask the user with | run `ghosthand doctor`, tell the user what's missing |

## Platform notes

- **Wayland with layer-shell** (niri, Hyprland, sway, river, KDE): the user's overlay draws the card (with what you will type) and the tag (`ghosthand overlay`, needs Quickshell).
- **X11, GNOME or no overlay:** a plain dialog (zenity, yad or kdialog) asks instead, and a notification says the pointer is in use. There is no cursor tag there.
- **Multiple monitors:** coordinates cover the output Ghosthand reports in `info`. If the pointer lands somewhere odd, say so; the user's compositor may need the virtual pointer (`ghosthand-pointer`) mapped to one output (see the docs).
- **Screenshots** use grim, maim, scrot, ImageMagick, spectacle or gnome-screenshot, whichever the system has.

## Troubleshooting

- `no write access to /dev/uinput`: the user needs the udev rule and the `input` group (`docs/setup.md` in the repo). Tell them; don't try `sudo`.
- Exit 5: the overlay isn't running and there's no dialog tool. `ghosthand doctor` shows which.
- The click lands in the wrong place: re-take a screenshot, compare its size to `ghosthand info`, and recompute. Don't just offset.
- Nothing happens on click: you probably need a `move` and a short `wait` first so the target sees the hover.

## Keeping this current

The skill, the overlay and the rules for your instructions file ship inside the binary and change between releases. After upgrading, `ghosthand skill install --all` refreshes the skill in every agent that is installed, and `ghosthand rules install` keeps the short rules block in your agents' global instructions files up to date (`ghosthand rules status` and `ghosthand doctor` say what is out of date). `ghosthand overlay` replaces a stale overlay by itself when it starts.

## If your host prefers MCP

`ghosthand mcp` serves the same thing over stdio with the tools `pointer` (which takes the same actions, keyboard included), `session`, `screenshot`, `screen_info` and `gate_status`. Same gate, same rules above apply. There is no tool to approve a request: that is always the user's job.
