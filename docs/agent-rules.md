# Agent rules

A skill is only read when an agent decides to load it. The global instructions file an agent reads on every start is read before its first call. `ghosthand rules` puts a short block there, so the two rules that matter most are in front of the agent from the start:

- **Ask once, not per call.** Before the first input action of any task that needs more than one call, start a session, work inside it, end it when done. Otherwise you click Allow again and again.
- **Never go around the gate.** No answering its own card, no `wtype`, `xdotool`, `ydotool`, no writing to `/dev/uinput`. A denial or a revoke means stop and tell you.

```sh
ghosthand rules                  # print the block (to paste somewhere yourself)
ghosthand rules status           # which instructions files have it, and whether it is current
ghosthand rules install          # add it, or update it in place, in every global instructions file that exists
ghosthand rules install FILE...  # or in the files you name
ghosthand rules install --dry-run
ghosthand rules remove           # take it out again, leaving the rest of the file as it was
```

It looks for the global instructions file of Claude Code, Codex, Antigravity and opencode (`ghosthand rules status` lists the ones it found). It only touches files that already exist, never creates one, and only changes the text between its own markers (`<!-- ghosthand:rules:begin ... -->` and `<!-- ghosthand:rules:end -->`). Files that are the same file through a symlink are written once. Run `ghosthand rules install` again after an upgrade; `ghosthand doctor` and `rules status` say when the block is out of date.

Over MCP the same advice is in the server's instructions and in the `pointer` tool description, so nothing needs installing.

Nothing here can approve anything. The block is advice to the agent; the card is still yours.
