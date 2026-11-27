# MCP

`ghosthand mcp` speaks MCP over stdio, one JSON-RPC message per line. No dependencies. Logs go to stderr, never stdout.

```json
{ "mcpServers": { "ghosthand": { "command": "ghosthand", "args": ["mcp"] } } }
```

## Protocol versions

It follows the [2026-07-28 spec](https://modelcontextprotocol.io/specification/2026-07-28/changelog), and still works with older clients. Which one it speaks is decided by the first message.

| Era | How it starts | What you get |
| --- | --- | --- |
| **2026-07-28** (current) | No handshake. Every request carries `params._meta["io.modelcontextprotocol/protocolVersion"]`. A client can call `server/discover`. | `resultType: "complete"` on every result, `ttlMs` and `cacheScope` on `server/discover` and `tools/list`, server info in `_meta`. |
| **2025-11-25** and older | `initialize`, then `notifications/initialized`. | Plain results, no `_meta` needed after `initialize`. The version is negotiated (2025-11-25, 2025-06-18, 2025-03-26, 2024-11-05). |

An unknown version in `_meta` gets `-32022` with the list of versions it supports. A modern request with no version is `-32602`. Notifications get no reply. Closing stdin shuts it down.

## Tools

| Tool | Needs approval | What it does |
| --- | --- | --- |
| `pointer` | yes | Runs `actions` (a list of strings, mouse and keyboard) with a `reason` shown on the card, along with what will be typed. Optional `who` and `countdown`. Returns text and `structuredContent` `{ ok, code, message, printed }`. |
| `session` | yes (`start`) | `action` `start` (needs a `reason`), `status` or `end`. One card for a whole stretch of work, then `pointer` calls inside the approved scope run without one. Optional `minutes`, `idle_minutes`, `scope` (`both`, `mouse`, `keyboard`) and `who`. Starting one that is already on and covers the request returns `already_on` with no card. |
| `screenshot` | no | A PNG image of the screen, or of a `region` `[x, y, w, h]`. |
| `screen_info` | no | The output's name, size and scale (`structuredContent`). |
| `gate_status` | no | `idle`, `asking`, `active`, `allowed`, `denied`, `timeout` or `revoked`. |

Every tool has a title, annotations (`pointer` is marked as acting on the world, the rest as read-only) and an input schema. The ones that return data have an output schema, and the structured result is also sent as text for clients that don't read it.

If you deny, or don't answer, `pointer` returns an error result. A well-behaved agent stops and tells you, rather than retrying in a loop. A tool that doesn't exist is a JSON-RPC `-32602`.

There is deliberately no tool to approve a request or switch the gate off.

## Limits

`pointer` blocks while it works, so a client's `notifications/cancelled` can't interrupt it. Use `ghosthand revoke` (bind it to a key) to stop a run.

## Prefer no MCP?

Install the [skill](../skills/ghosthand/SKILL.md) (`ghosthand skill install`) and let the agent use the CLI.
