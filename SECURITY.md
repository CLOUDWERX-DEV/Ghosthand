# Security

## What the gate is, and isn't

Ghosthand's gate is a courtesy between you and an agent that's behaving. It asks first, shows what's happening, and lets you take control back. It is **not** a security boundary.

Anything that can write to `/dev/uinput` can create its own virtual pointer without going through Ghosthand. If your agent has a shell running as your user, and your user is in the `input` group, it can do that. So:

- Treat the gate as a seatbelt, not a vault.
- If you need a hard boundary, run the agent in a container, VM or user that doesn't have `/dev/uinput`, and let only Ghosthand's MCP server (running outside it) have access.
- `ghosthand revoke` stops Ghosthand's own pointer. It can't stop a second device someone else made.

## The keyboard is the riskier half

A pointer can click things. A keyboard can run commands: type into a terminal and press Enter and it's a shell. So:

- The card says "keyboard" and shows exactly what will be typed or pressed. Read it before you Allow.
- "Allow for N min" for the mouse never covers the keyboard. They're granted separately.
- `"keyboard": false` in the config switches every keyboard action off.
- Keys go to the focused window, and the agent can't aim them. `focus` tells it where they'll land, but it can't be forced to look.

## What it does protect

- The mode, countdown and grant length are read by the overlay from its own config. A program asking for the pointer can't pass a mode that skips the card.
- An unreachable overlay is a refusal, not a pass.
- A locked session, or one that can't be checked, is refused.
- The MCP server has no tool that approves a request or turns the gate off.
- Screenshots need no approval because they don't move anything, but they do show your screen. Don't point an agent you don't trust at it.

## Reporting a problem

Email mail@cloudwerxlab.com, or open a private security advisory on the GitHub repo. Please don't post details in a public issue first. I'll reply as soon as I can.
