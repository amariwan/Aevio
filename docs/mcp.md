# Aevio MCP server

`aevio mcp serve` starts a local Model Context Protocol server over stdin/stdout.
It uses the same typed local IPC and database access as the Aevio CLI, so it
does not create a second data or authentication service.

The first server surface is intentionally read-only:

- `aevio_status`
- `aevio_list_projects`
- `aevio_list_categories`
- `aevio_list_clients`
- `aevio_list_time_entries`
- `aevio_pending_reviews`
- `aevio_report`

The server also exposes the `aevio://context` resource and the
`aevio_daily_review` prompt for MCP clients that support resources and prompts.

`aevio_list_time_entries` is bounded to the last 30 days by default and caps
the result at 200 rows. Pass `startMs`, `endMs`, `status`, `search`, and
`limit` when a narrower query is needed. Mutating tools are not exposed until
there is an explicit approval and audit contract.

## Client configuration

For a local MCP client that supports stdio, configure the installed executable:

```json
{
  "mcpServers": {
    "aevio": {
      "command": "aevio",
      "args": ["mcp", "serve"]
    }
  }
}
```

For a development checkout, point `command` at the built binary or use the
repository's normal Cargo runner. The server supports the MCP initialize,
tools/list, tools/call and ping methods; notifications do not receive a
response, as required by JSON-RPC.

The Agent page uses the same privacy boundary: it prefers the local Swift
Foundation Model sidecar when available and reports a deterministic local
read-only fallback when the model is unavailable.

## Project Brain MCP

Repository-aware agents can use the read-only Project Brain facade without a
second index service:

```json
{
  "mcpServers": {
    "aevio-project-brain": {
      "command": "node",
      "args": ["scripts/project-brain.mjs", "mcp"]
    }
  }
}
```

The facade delegates to the existing `aevio-knowledge`, ProjectAtlas,
GitNexus, and Graphify indexes. Every result includes branch/HEAD freshness,
confidence, unresolved warnings, and source provenance. It does not mutate
application data or accept write tools.
