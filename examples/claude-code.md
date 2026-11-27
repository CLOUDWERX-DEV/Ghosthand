# Using Ghosthand from Claude Code

```sh
claude mcp add ghosthand -- ghosthand mcp
```

Then tell it something like: "Open the bar menu and take a screenshot, use the pointer tool." A card appears, you click Allow, and it goes.

Or skip MCP and let it call the CLI from a shell:

```sh
ghosthand --who Claude --reason "Checking the bar menu" 'jump 800 26' 'rclick' 'wait 0.6' 'shot /tmp/menu.png'
```
