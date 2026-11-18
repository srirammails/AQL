# Changelog

AQL has one version number. The language version **v0.N** is described by its spec
in [`spec/`](spec/); the crates in this repository (`aql-parser`, `clawdb`,
`clawdb-server`, `clawdb-wasm`) are released together as **0.N.x**, where 0.N.x
parses and implements language v0.N. A patch release (0.N.1, …) fixes the parser or
the reference implementation without changing the language. Every release is tagged
`v0.N.x` in git.

From v0.6 to v0.12 each version only added to the language: every older statement
parsed as before. v0.13 is a clean break: one smaller language replaces v0.5 and the
v0.6–v0.12 additions, and older statements change form (see the 0.13.0 entry).

| Language | Release | Date | Spec | Conformance suites |
|---|---|---|---|---|
| v0.13 | 0.13.0 | 17 Nov 2026 | [v0.13 specification](spec/AQL_SPEC_v0.13.md) (replaces all below) | all 34 rewritten in v0.13; new `33_branch_fork_merge`, `34_use_and_namespaces` |
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

**Language v0.13: one small language.** [`spec/AQL_SPEC_v0.13.md`](spec/AQL_SPEC_v0.13.md)
replaces v0.5 and the v0.6–v0.12 additions. Not compatible with them: statements
change form, and the spec's §9 maps every old form to the new one.

- **13 verbs + `USE`**, each on every memory type: `RECALL STORE UPDATE FORGET LINK
  UNLINK` (memory), `BRANCH FORK MERGE` (namespaces), `PIPELINE SUBSCRIBE UNSUBSCRIBE
  PUBLISH` (flow), and `USE` (session).
- **`RECALL` is the only read.** `LOOKUP`, `LOAD` and `SCAN` are gone; the engine
  picks the access path. `KEY x = v` is `WHERE x = v`; similarity is
  `LIKE "text" | $var` with `CONFIDENCE` (was `PATTERN … THRESHOLD`, `MIN_CONFIDENCE`).
- **`BRANCH` / `FORK` / `MERGE` on namespaces.** A branch is a namespace that reads
  through its base at a point in time (`AS OF`); a fork is a full copy; `MERGE`
  applies one side's changes to the other, failing on conflicts unless `PREFER
  SOURCE | TARGET`. Replaces the `BRANCH "b"` modifier. Dropping a branch is
  `FORGET FROM ALL NAMESPACE "b"`.
- **Aliases instead of `MERGE ENTITY`.** `UPDATE … (aliases = [...])` then `FORGET`
  the duplicate; reads and links by the alias reach the survivor.
- **No facts, no `VALID AT`.** Relationships are `LINK`s, values are fields;
  `UPDATE` keeps history for `AS OF "<time>"` and `WITH HISTORY [BETWEEN … AND …]`
  (`AS OF 2d AGO` is gone). `UPDATE LINK` is `LINK` (create or set the weight).
- **`USE "ns"` and scripts.** Statements separated by `;` form a script; a `USE` gives
  the later statements that name no namespace that one, and a statement's own
  `NAMESPACE` wins. A script's answer is its last statement's; a failing statement
  stops it and is named by position. In code: `aql_parser::parse_script`.
- **`NAMESPACE ("a", "b")`** on `RECALL` and `SUBSCRIBE RECALL`: one read across
  several namespaces. Writes, links and topics are in one namespace.
- **Keyword cuts** (about 60 keywords left): `LINK … TO` is `LINK … INTO`; `MIN_WEIGHT`
  is `WEIGHT` (a minimum), links are read in both directions (`OUTBOUND / INBOUND /
  BOTH` gone), `TYPE IN (…)` is `TYPE ("a", "b")`; `WINDOW TOP n BY f` is
  `ORDER BY f LIMIT n`; `WATCH` / `UNWATCH` are `SUBSCRIBE RECALL` / `UNSUBSCRIBE`;
  `REFLECT … THEN` is a `PIPELINE` with a write last; `ACK`, `DEBOUNCE`, `FROM NOW`,
  `FROM OFFSET`, `KINDS` and `SCOPE private|shared|cluster` are gone (server API and
  settings, or `NAMESPACE` / `IN SCOPE`); `TIMEOUT` is on `PIPELINE` only.

**Conformance.** 34 suites, 406 tests, all written in v0.13; new suites
`33_branch_fork_merge` and `34_use_and_namespaces` (server only).

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
