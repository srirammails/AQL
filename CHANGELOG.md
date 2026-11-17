# Changelog

AQL has one version number. The language version **v0.N** is described by its spec
in [`spec/`](spec/); the crates in this repository (`aql-parser`, `clawdb`,
`clawdb-server`, `clawdb-wasm`) are released together as **0.N.x**, where 0.N.x
parses and implements language v0.N. A patch release (0.N.1, …) fixes the parser or
the reference implementation without changing the language. Every release is tagged
`v0.N.x` in git.

Each version after v0.5 only adds to the language: every older statement parses as
before, and its AST serializes to the same JSON.

| Language | Release | Date | Spec | Conformance suites |
|---|---|---|---|---|
| v0.13 | 0.13.0 | 17 Nov 2026 | [v0.13 additions](spec/AQL_SPEC_v0.13_additions.md) | `32_merge_namespace` |
| v0.12 | 0.12.0 | 17 Nov 2026 | [v0.12 additions](spec/AQL_SPEC_v0.12_additions.md) | `31_namespace_statements` |
| v0.11 | (in 0.12.0) | 4 Nov 2026 | [v0.11 additions](spec/AQL_SPEC_v0.11_additions.md) | `30_branch` |
| v0.10 | (in 0.12.0) | 30 Oct 2026 | [v0.10 additions](spec/AQL_SPEC_v0.10_additions.md) | `27_time`, `28_merge`, `29_scoped_keys` |
| v0.9 | (in 0.12.0) | 30 Oct 2026 | [v0.9 additions](spec/AQL_SPEC_v0.9_additions.md) | `24_group_by`, `25_window_episodic`, `26_update_null` |
| v0.8 | (in 0.12.0) | 29 Oct 2026 | [v0.8 additions](spec/AQL_SPEC_v0.8_additions.md) | `23_graph` |
| v0.7 | (in 0.12.0) | 26 Oct 2026 | [v0.7 additions](spec/AQL_SPEC_v0.7_additions.md) | `22_link_update` |
| v0.6 | (in 0.12.0) | 9 Oct 2026 | [v0.6 additions](spec/AQL_SPEC_v0.6_additions.md) | `18_subscriptions`, `19_watch`, `20_in_scope` |
| v0.5 | 0.1.0 | 1 Apr 2026 | [v0.5 specification](spec/AQL_SPEC_v0.5.md) (the base) | `01`–`17`, `21` |
| v0.2–v0.4 | — | Mar 2026 | [`spec/`](spec/) (superseded) | — |

Versions v0.6 to v0.11 were developed on the `adb-v0.6` branch and are first
released, with v0.12, as 0.12.0. Before 0.12.0 the crates were all at 0.1.0.

## 0.13.0 — 17 Nov 2026

**Language v0.13: `MERGE ENTITY` can name its namespace** — the last statement
that couldn't.

- `MERGE ENTITY "<dup>" INTO "<survivor>" NAMESPACE "<ns>"`: both entities are the
  namespace's; the merged key is an alias within that namespace only.
- The AST carries it as `namespace` on the `Merge` statement; absent when not given.

**Conformance.** 32 suites, 334 tests (suite `32_merge_namespace`, server only).

## 0.12.0 — 17 Nov 2026

**Language v0.12: every statement can name its namespace.**

- `LINK`, `UPDATE LINK` and `UNLINK` take `NAMESPACE "<ns>"` (both ends in that
  namespace) and, after it, `BRANCH "<name>"`.
- `SUBSCRIBE`, `ACK` and `UNSUBSCRIBE` take `NAMESPACE "<ns>"` (`PUBLISH` already
  did): a namespace's own topics and subscriptions.
- The AST carries `namespace` (and `branch` on the link statements); absent when not
  given.

**Language v0.11: memory branches.** `BRANCH "<name>"` on reads and writes.

**Language v0.6–v0.10**, first released here: subscriptions and continuous queries
(`SUBSCRIBE`, `PUBLISH`, `ACK`, `WATCH`, `IN SCOPE`), `UPDATE LINK` / `UNLINK`, graph
reads and facts, `GROUP BY`, `WINDOW` on episodic recall, `UPDATE field = null`, fact
validity and history reads (`AS OF`, `VALID AT`, `WITH HISTORY`), `MERGE ENTITY`, keys
per scope. History reads are allowed on every memory type but WORKING.

**Reference implementation.** ClawDB and the WASM playground implement v0.5 and v0.7
(`UPDATE LINK` / `UNLINK` in the playground); they refuse, with an error naming the
version, what only ADB implements: subscriptions and `WATCH` (v0.6), `GROUP BY` and
`WINDOW` on `RECALL` (v0.9), history reads and `MERGE ENTITY` (v0.10), `BRANCH`
(v0.11) and `NAMESPACE` on links (v0.12).

**Conformance.** 31 suites, 321 tests. Suites marked `requires: server` run against
a server (`node tests/run-tests.mjs --url …`).

**Fixes.** A parser test still expected `AS OF` on EPISODIC to be refused.

## 0.1.0 — 1 Apr 2026

Language v0.5: `FROM` / `INTO`, `WINDOW`, `AGGREGATE`, `HAVING`; the parser,
ClawDB, the WASM playground and the first conformance suites.
