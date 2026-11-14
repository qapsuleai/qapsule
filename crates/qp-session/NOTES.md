# qp-session — spec issues found and decisions taken

Reading of `docs/PROTOCOL.md` (merged up to rd1/t3-protocol d718c7b) where it was silent,
ambiguous or contradictory. Rule used: the simplest reading consistent with the types.

## Conformance transcripts

All 16 official transcripts pass on `inproc` and `serve`. None contradicted the spec.
One spec example disagrees with transcript 02: §4.3 shows `insert_layer` `"index": 4`
for `show: {with: "rsi14"}` in a view with five layers; transcript 02 expects `5`. I
append the layer at the end (`index = number of layers`), as transcript 02 does. The
§4.3 example should say `5`. The §4.1 welcome (`seq: 4`) and §4.6 zoom (`170..199`)
examples are illustrative only.

## Boundary and errors

1. **Paths inside tagged enums.** `serde_path_to_error` loses the path at the first
   internally tagged enum, because serde buffers its content (checked: an unknown
   field inside an op reports path `.`). Every protocol message is such an enum. So on a
   serde failure the authority walks the message alongside the JSON Schema generated
   from the same types (`schemars`), picks `oneOf` branches by their tag constant, and
   reports the first violation with a JSON pointer, `expected` and `did_you_mean`. If
   the walker finds nothing, it falls back to serde's message and the
   `serde_path_to_error` path.
2. When several schema errors exist, the walker checks unknown fields first, then
   missing required fields, then the fields in key order. The reported error can
   differ from the one serde hit first. Both are real errors.
3. Hitting the JSON nesting limit (serde_json, 128) gives `limit`, not `parse`.
4. `max_string` is checked on string values, not on object keys, after parsing.
   `max_bars` also applies to inline bars in `put_series`.
5. Handshake errors carry `path` (`/protocol`, `/principal`) and, for unknown
   principals, `did_you_mean`.

## Line transport (`qp serve`)

6. **Sending raw text.** The spec gives no way to send invalid JSON through `qp serve`,
   since the frame itself must be JSON. Decision: if `msg` is a JSON **string**, it is
   the raw text of the message. A `ClientMsg` is always an object, so this is
   unambiguous. The serve conformance transport sends every `send_raw` this way.
7. Frame errors: a non-JSON line gives `parse` on conn `""`; a missing or non-string
   `conn` gives `schema` on conn `""`; an unknown frame field or a missing `msg` gives
   `schema` on the frame's conn. Frame lines are limited to `max_message_bytes + 4096`.
8. Added `qp serve --root <dir>` (default `.`). Document paths in the init are
   relative to it and confined to it (absolute paths, `..` and paths that canonicalize
   outside the root are refused).

## Commands and ids

9. `created` lists every item the command creates, whether the id came from the
   client or was assigned. `n` in `<kind>-<seq>-<n>` counts all created items of the
   entry (transcript 15: `["sma20","sma50","intent-1-2"]`). An existing series or view
   replaced by `put_series`, `put_view` or `show_finding` is not listed.
10. `accept_proposal` and `undo` must be the only op of their command (`invalid`
    otherwise), because `LogEntry` has one `author`/`approved_by` and one `undoes`.
11. In `auto` mode, a command with any op the speaker may not write becomes a proposal
    as a whole, if every op is proposable. Otherwise it is `forbidden` at the first
    offending op.
12. Study params are stored in full, with defaults merged (`add_study`, `set_params`,
    `put_series`). `add_study` copies `instrument` and `interval` from its input.
    Studies take exactly one bar series as input.
13. `show.with` must name a series that exists (`not_found`) and is drawn in the view
    (`invalid`). `show.panel` may be at most max panel + 1.
14. A layer on a panel that has no y scale yet (new panel from `add_study` or
    `add_layer`) is written as a `put_view` that extends `y` with linear scales, not as
    an `insert_layer`. `put_view` refuses layers whose panel has no y scale.
15. An inline series must carry bars. Points data gives `unsupported`.
16. `remove`: besides the spec's series rule, a fact used by findings, a finding with
    capsules, and a view used by a capsule are `in_use`. Proposals cannot be removed
    (`invalid`), because they stay in the document as a trace.
17. `add_finding` needs at least one fact. `add_narrative` needs at least one step, with
    `at_ms` strictly increasing. Narrative and directive targets (view, series, item)
    must exist.
18. `approve_finding`/`reject_finding` on a finding without a capsule only records the
    ledger entry.
19. `update_finding` keeps the finding's original provenance; the log records who
    changed it. Capsule `revision` is not bumped.

## Proposals

20. The accepted entry's `request` is the proposal's ops. Its `reason` is the accept
    command's reason, falling back to the proposal's. Inside the expansion the speaker
    is the proposer.
21. If acceptance fails, the error keeps the inner code, `path` is `/ops/0`, and the
    message includes the inner path within the proposal.
22. The proposer principal for `proposal` events is found by comparing `Author` values
    against the principal registry, falling back to the author's name.
23. Expiry happens when `now >= expires_at`. Each expired proposal gets its own entry
    (author `qp`, document order) and no `decided_by`.

## Engine entries, conditions, notifications

24. Invalidation is evaluated after entries containing `put_series`, `append_bars`,
    `truncate_bars` or `set_last_bar` (all live capsules) or `put_finding` (that
    finding's live capsules). It writes one engine entry per capsule, with no `cause`.
    `set_capsule_state` is not a trigger, so undoing an invalidation sticks.
25. `finding_state` events are sent for `set_capsule_state` changes only, not for new
    capsules.
26. Edge baseline: an intent or condition subscription records its current value when
    it appears (creation, activation, subscription) and does not fire if it is already
    true. Conditions that cannot be evaluated count as false. Text triggers are never
    evaluated. A text condition in `subscribe` is `invalid`.
27. Watchers are evaluated after every entry, including proposal and engine entries.
    `condition_met`/`intent_triggered` carry the seq of the entry that made the
    condition true. Order: all `condition_met`, then all `intent_triggered`.

## Undo and base

28. An entry counts as undone if an undo entry targets it and that undo entry is not
    itself undone (computed recursively from the end of the log).
29. `undo_conflict` checks later entries that are neither undone nor undo entries
    (an undo entry cancels out with its target). Without this, "undo, then undo again"
    would always conflict with the first undo.
30. `base`: entries after `base` that are undone are skipped, and so are undo entries
    whose target is also after `base` (the pair cancels out).
31. Nothing to undo gives `undo_conflict`. A target that is already undone gives
    `undo_conflict`. An unknown seq gives `not_found`. `forbidden` is checked first.
32. A decision effect (§8.3, needs `decide`) is detected in the target's changes or
    inverse: `set_capsule_state` to approved or rejected, `push_ledger`/`pop_ledger`,
    or `put_proposal` with a state other than pending. Undoing the undo of a rejection
    is therefore also a decision.
33. **Not implemented:** "a `base` from another epoch is a `conflict`" (§8.4).
    `command` carries no epoch, so the authority cannot tell.

## Queries, input, presence, presenter

34. Range: `from_bar`/`to_bar` and `from_t`/`to_t` may be given alone (open ends).
    Mixing forms is `invalid`. Ranges are clipped to the bars.
35. `at` with neither `bar` nor `t` uses the last bar. Both is `invalid`. `t` must match
    a bar exactly. `studies` covers study series whose first input is that bar series.
36. `reduce`: ties in `min`/`max` resolve to the first occurrence. `count`'s `value` is
    the count as a number, without `bar`.
37. `look` past the last bar is `invalid` (not clipped). `pan` clamps and keeps the
    width. `zoom`: width = max(2, round(width / factor)), capped at the bar count. The
    anchor keeps its relative position r = (anchor - from)/(width - 1). Rounding is
    half away from zero, and the result is clamped to the bars.
38. `tap`: the panel is valid if it is below `y.len()` or used by a layer. Focus is
    `{view, panel, from_bar, to_bar}` (no series). The viewport does not change.
39. Attention viewport: among the principal's screen connections, the one with the
    latest input, else the last opened. With no input it is the initial range of the
    document's first view. `at` is the clock at that input.
40. Screen `capsules`: capsules of any state whose finding has any fact with a bar in
    the visible range.
41. `present`: a "live finding" is one with a capsule in state `live`. When no view is
    attached, the presenter's view (not written to the document) gets id
    `present-<finding>`. `spark` holds the main output only and is sent only when
    `series_data` is true and `max_points > 0`. A finding without facts cannot be
    presented with `show_finding` (`invalid`).
42. `surface.max_depth` lowers `depth` only; the other profile fields keep the surface
    kind's values.
43. Attention directives: `steps` must not be empty. A client-given `id` is used, and
    the `att-<n>` counter advances anyway.
44. The `feed` role has no `read`, so it cannot query, subscribe or send input. `clock`
    from a non-host is `forbidden`; clock errors have no `id`.

## Changes to other crates

45. `qp-schema`: added `#[allow(clippy::large_enum_variant)]` on `ServerMsg`.
    `cargo clippy -D warnings` failed on it as merged from rd1/t3-protocol. This is
    lint-only: types and serialization are unchanged. `qp-compute`/`qp-render`: no
    changes.
46. Undoing a proposal-creation entry deletes the proposal. The proposer cannot do this
    (no `write`); an owner can.
