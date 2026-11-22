# qp-mcp — notes

The MCP adapter for `qp/0` (`rounds/RD1/EMBEDDING.md` §1). The adapter adds no state
to the protocol: each tool call becomes one `qp/0` client message, handed to
`qp_session::Session::handle_raw`. The protocol reply is the tool result.

## MCP revision followed

I read these pages on modelcontextprotocol.io on 2026-11-14:

- **2026-07-28** (the current revision): `server/tools`, `basic` (messages,
  statelessness, `_meta`, error codes), `basic/versioning`, `server/discover`,
  `basic/transports/stdio`, `server/utilities/logging`.
- **2025-11-25** `basic/lifecycle`, for `initialize`, since most clients still send it.

The server serves both eras. A modern request (per-request `_meta`) is served
statelessly. `initialize` gets a legacy answer.

## Choices

1. **Versions.** `server/discover` lists `2026-07-28`, `2025-11-25` and `2025-06-18`.
   `initialize` echoes `2025-11-25` or `2025-06-18` if asked for one of them, and
   otherwise answers `2025-11-25`. If a request's
   `_meta["io.modelcontextprotocol/protocolVersion"]` names some other version, the
   reply is `-32022` with `data.supported`/`requested`. **Lenient:** I accept requests
   that have no `_meta` instead of rejecting them with `-32602` as 2026-07-28 requires,
   because legacy sessions and simple clients leave it out.
2. **Results** always carry `resultType: "complete"`. Legacy clients ignore the extra
   field.
3. **Identity lives in the adapter**, not on the wire (MCP is stateless). The adapter
   is built for one principal and surface, sends its own `hello` when it is built, and
   fails to start if the authority refuses it. The `welcome` is exposed in
   `_meta["dev.qapsule/welcome"]` of `initialize` and `server/discover`.
4. **Tools are generated** from `qp_schema::json_schema("ClientMsg")`:
   - one `op_<name>` per `Op` variant: the variant schema without `op`, plus `mode`,
     `base` and `reason` copied from the `command` schema;
   - one `query_<kind>` per `Query` variant: the variant schema without `kind`;
   - then `command`, `query`, `subscribe`, `unsubscribe`, `attend`: the `ClientMsg`
     variant without `type`, with `id` optional.

   Order is deterministic: ops, then queries, both in schema order, then the generic
   tools. Each input schema carries only the `$defs` it reaches. Descriptions are the
   schema's doc comments. A test checks that no op field is named `mode`, `base` or
   `reason`.
5. **Request id.** Op and query arguments can have their own `id` field (`add_study.id`,
   `get.id`), so the `qp/0` request id cannot be a plain argument there. It travels in
   `tools/call` `params._meta["dev.qapsule/id"]`. The generic tools take it as their
   `id` argument. If no id is given, one is generated (`mcp-<n>`, counted per adapter).
6. **outputSchema** is kept slim:
   `{"type":"object","properties":{"type":{"enum":["ack","result","error"]}}}`. The full
   `ServerMsg` schema on each of the 37 tools would bloat `tools/list`, which is already
   about 110 KB. `qp schema export` writes the full schema.
7. **Errors.** A protocol `error` is a tool execution error: `isError: true`, and
   `structuredContent` is the error message with `path`, `expected` and
   `did_you_mean`. The `path` refers to the built `qp/0` message, e.g.
   `/ops/0/params`. JSON-RPC errors are used only for framing problems:
   - `-32700` parse error;
   - `-32600` invalid request (batches are refused);
   - `-32601` unknown method;
   - `-32602` unknown tool or non-object `arguments`.
8. **Notifications.** `notify` messages for the adapter's connection are queued in the
   hub. They go out as `notifications/message`
   (`level: info`, `logger: qapsule`, `params.data` = the `notify`), written before the
   response to the adapter's next request. `params._meta["dev.qapsule/order"]` is the
   hub-wide emission index, which lets a client with several adapters merge the streams
   back into authority order. The in-process API is `McpServer::take_notifies`.
   **Deviation:** 2026-07-28 deprecates `notifications/message`, scopes it to a request
   and requires that request's `_meta` to carry `io.modelcontextprotocol/logLevel`. I
   send it without `logLevel`, as the task asks. The 2026-07-28-native mapping would be
   a `subscriptions/listen` stream plus the `log` query (EMBEDDING §1.1). That is left
   as a suggestion.
9. **stdio** (`qp mcp serve`): one JSON-RPC message per line, nothing else on stdout,
   exits at EOF. (RD2/A62: the RD1 one-adapter `qp mcp --init --principal` mode, with no
   host, no `clock` and the decision tools, is removed; [`McpServer::connect`] remains
   only for the in-process conformance runner.)
10. **Threading.** `SharedHub = Rc<RefCell<Hub>>` (single-threaded, like
    `qp serve`). An HTTP host would need `Arc<Mutex<_>>`.

## Conformance over MCP

`qp conform … --transport mcp` (`qp_mcp::conform`) gives each transcript connection its
own `McpServer` over one in-process `Hub`. The translation rules are in the module docs.

**Not carried over MCP**: these go straight to the session and are counted as "direct"
in the runner output:

- `input`: raw interaction stays in the core.
- `clock`: host only.
- `send_raw`: not a JSON message.
- a message sent before the connection's `hello`: an adapter always says `hello` first.
- a message whose `id` is not a string: an MCP tool call always has a request id.

Result: all 16 transcripts in `fixtures/protocol/conformance/` and the 3 in
`crates/qp-session/tests/transcripts/` pass over `mcp`. That is 90 tool calls,
46 hellos and 17 direct steps.

## qp-session changes

None. The adapter uses only public API: `Session::new`, `Session::handle_raw`,
`document`, `log`, `initial_document`, plus `conform::{Transport, run_steps, load_init,
FinalState, TranscriptOutcome}`.
