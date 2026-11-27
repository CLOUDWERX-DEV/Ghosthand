# Using Ghosthand from an agent

Two ways, pick whichever your agent likes. Both go through the same gate.

## From the shell, with the skill (no MCP)

```sh
ghosthand skill install                    # puts the skill in ~/.claude/skills/ghosthand
ghosthand skill install ~/.agents/skills   # or any skills folder your agent reads
```

The agent then runs `ghosthand` itself:

```sh
ghosthand --who Claude --reason "Checking the bar menu" 'jump 800 26' 'rclick' 'wait 0.6' 'shot /tmp/menu.png'
```

## Over MCP

Any client that can start a stdio server works. The command is always `ghosthand mcp`.

```json
{ "mcpServers": { "ghosthand": { "command": "ghosthand", "args": ["mcp"] } } }
```

For Claude Code: `claude mcp add ghosthand -- ghosthand mcp`. For Codex, add it under `[mcp_servers.ghosthand]` in `~/.codex/config.toml` with `command = "ghosthand"` and `args = ["mcp"]`.

Then ask for something like "open the bar menu and take a screenshot, use the pointer tool". A card appears, you click Allow, and it goes.
