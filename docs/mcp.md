# MCP

`ghosthand mcp` speaks MCP over stdio. No dependencies.

```json
{ "mcpServers": { "ghosthand": { "command": "ghosthand", "args": ["mcp"] } } }
```

| Tool | Needs approval | What it does |
| --- | --- | --- |
| `pointer` | yes | Runs actions (`actions`, a list of strings) with a `reason` shown on the card. Optional `who` and `countdown`. |
| `screenshot` | no | A PNG of the screen or a `region` `[x, y, w, h]`. |
| `screen_info` | no | The output's name, size and scale. |
| `gate_status` | no | `idle`, `asking`, `active`, `allowed`, `denied`, `timeout` or `revoked`. |

If you deny, or don't answer, `pointer` returns an error. A well-behaved agent stops and tells you, rather than retrying in a loop.

There is deliberately no tool to approve a request or switch the gate off.
