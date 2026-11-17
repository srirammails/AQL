# AQL v0.13 Conformance Test Suite

This directory contains the official conformance test suite for AQL (Agent Query
Language) v0.13, release 0.13.0: the 13-verb language of
[AQL_SPEC_v0.13.md](../spec/AQL_SPEC_v0.13.md), which replaces v0.5 and the
v0.6–v0.12 additions.

**406 tests in 34 suites**: 246 that run on any implementation and 160 that need a
server (`requires: server`). The suites were rewritten from the v0.5–v0.12 syntax
(LOOKUP / LOAD / SCAN → RECALL, LINK … TO → INTO, facts → links and fields,
MERGE ENTITY → aliases, WATCH → SUBSCRIBE RECALL, REFLECT → PIPELINE, …); each
suite's header says what changed and which tests were removed because v0.13 has
no equivalent. Suite 33 (BRANCH / FORK / MERGE on namespaces) is new.

## Purpose

Any AQL implementation (Rust, Python, TypeScript, Java, etc.) can run this test suite to verify correctness against the specification.

## Structure

```
tests/
├── README.md                 # This file
├── fixtures/
│   └── seed.aql              # Canonical seed data - run BEFORE any test suite
├── suites/
│   ├── 01_scan.yaml          # RECALL FROM WORKING, WINDOW, ORDER BY … LIMIT (was SCAN)
│   ├── 02_recall.yaml        # RECALL: comparators, AND/OR, ORDER BY, LIMIT, FROM ALL
│   ├── 03_lookup.yaml        # RECALL by key (was LOOKUP)
│   ├── 04_load.yaml          # RECALL FROM TOOLS (was LOAD)
│   ├── 05_store.yaml         # STORE
│   ├── 06_update.yaml        # UPDATE
│   ├── 07_forget.yaml        # FORGET
│   ├── 08_link.yaml          # LINK … INTO …
│   ├── 09_with_links.yaml    # WITH LINKS
│   ├── 10_follow_links.yaml  # FOLLOW LINKS
│   ├── 11_reflect.yaml       # multi-source PIPELINE, write last (was REFLECT)
│   ├── 12_pipeline.yaml      # PIPELINE, TIMEOUT
│   ├── 13_aggregate.yaml     # AGGREGATE
│   ├── 14_having.yaml        # HAVING
│   ├── 15_scope_namespace.yaml # NAMESPACE isolation
│   ├── 16_dotted_fields.yaml # metadata.* paths
│   ├── 17_validation.yaml    # refusals, including the replaced v0.5–v0.12 forms
│   ├── 18_subscriptions.yaml # SUBSCRIBE TOPIC / PUBLISH / UNSUBSCRIBE (server only)
│   ├── 19_watch.yaml         # live queries: SUBSCRIBE RECALL … (server only)
│   ├── 20_in_scope.yaml      # IN SCOPE (server only)
│   ├── 21_edge_cases.yaml    # edge cases
│   ├── 22_link_update.yaml   # LINK create-or-update, UNLINK, WEIGHT minimum
│   ├── 23_graph.yaml         # FOLLOW LINKS DEPTH, both directions, relationships as links
│   ├── 24_group_by.yaml      # GROUP BY
│   ├── 25_window_episodic.yaml # WINDOW on EPISODIC
│   ├── 26_update_null.yaml   # UPDATE field = null
│   ├── 27_time.yaml          # AS OF "<time>", WITH HISTORY [BETWEEN]
│   ├── 28_merge.yaml         # aliases (UPDATE aliases + FORGET; was MERGE ENTITY)
│   ├── 29_scoped_keys.yaml   # keys per scope (server only)
│   ├── 30_branch.yaml        # pointer: the BRANCH "b" modifier is gone (server only)
│   ├── 31_namespace_statements.yaml # NAMESPACE on links, topics, live queries (server only)
│   ├── 32_merge_namespace.yaml      # aliases within a NAMESPACE (server only)
│   ├── 33_branch_fork_merge.yaml    # BRANCH / FORK / MERGE on namespaces (server only)
│   └── 34_use_and_namespaces.yaml   # USE, scripts, NAMESPACE ("a", "b") (server only)
└── coverage/
    └── coverage_matrix.md    # Grammar production → Test ID mapping
```

## How to Use

### 1. Load Seed Data

Before running any test suite, execute all statements in `fixtures/seed.aql`:

```bash
# Your implementation should run these first
cat fixtures/seed.aql | your-aql-runner
```

The PROCEDURAL STORE returns a UUID - save it as `$PROC_ID` for later tests.

### 2. Run Test Suites

Each YAML file contains tests in this format:

```yaml
suite: RECALL
spec_section: "§6"
tests:
  - id: T02-01
    name: "WHERE = string"
    query: |
      RECALL FROM EPISODIC WHERE bid_id = "e-001" RETURN bid_id, campaign
    expect:
      success: true
      count: 1
      contains:
        - bid_id: "e-001"
          campaign: "summer_2026"
```

### 3. Interpret Results

| Field | Description |
|-------|-------------|
| `success` | `true` = query should succeed, `false` = should return error |
| `count` | Exact number of records expected |
| `count_gte` / `count_lte` | Minimum/maximum record count |
| `contains` | Records that MUST be in results (partial match) |
| `error_contains` | Substring that must appear in error message |
| `links_count` | Exact number of links on the returned records (WITH LINKS) |
| `links_contains` | Links that MUST be present (partial match; numbers within 1e-6) |
| `links_present` | Whether the returned records carry any links |
| `links_type_only` | Every returned link has this type |
| `fields` | Values at dotted paths of the result (`data.type: "snapshot"`, `data.result.data.count: 2`); `${name}` is replaced by a captured value |
| `fields_exist` | Dotted paths that must be present |

A test may also `capture: {name: path}` a value from its result (a subscription or
live-query id, a record's `_recorded_at` time) for later queries and `fields` as
`${name}`. A suite with `requires: server` uses engine-only statements
(subscriptions, scopes, branches) and is skipped unless the runner targets a
server (`node run-tests.mjs --url http://host:port`).

## Test Categories

| Suite | Tests | Coverage |
|-------|-------|----------|
| 01_scan | 5 | RECALL FROM WORKING: WINDOW LAST n / duration, ORDER BY … LIMIT, BETWEEN on time, WINDOW on EPISODIC |
| 02_recall | 20 | All comparators, AND/OR, ORDER BY, LIMIT, FROM ALL |
| 03_lookup | 4 | RECALL … WHERE x = v on SEMANTIC, PROCEDURAL, WORKING |
| 04_load | 2 | RECALL FROM TOOLS: ranking, task |
| 05_store | 8 | All memory types, TTL, NAMESPACE, STORE INTO ALL refused |
| 06_update | 10 | Additive fields, version increment, TTL race |
| 07_forget | 4 | WORKING / EPISODIC, idempotency, FROM ALL WHERE |
| 08_link | 6 | LINK … INTO across types, error handling |
| 09_with_links | 2 | Links payload in responses |
| 10_follow_links | 2 | Single-hop traversal |
| 11_reflect | 4 | Multi-source PIPELINE, RECALL FROM ALL, STORE / UPDATE as the last stage |
| 12_pipeline | 6 | Named/anonymous, TIMEOUT (PIPELINE only), {var} binding |
| 13_aggregate | 10 | COUNT, SUM, AVG, MIN, MAX on all types |
| 14_having | 4 | Alias resolution, suppress, multi-alias |
| 15_scope_namespace | 3 | NAMESPACE isolation |
| 16_dotted_fields | 3 | metadata.namespace, metadata.version |
| 17_validation | 13 | ALL on writes / links, FORGET without WHERE names its NAMESPACE, TIMEOUT optional, the replaced v0.5–v0.12 forms refused |
| 18_subscriptions | 9 | SUBSCRIBE TOPIC [WHERE], PUBLISH, UNSUBSCRIBE (server only) |
| 19_watch | 11 | SUBSCRIBE RECALL snapshots (key, ORDER BY, aggregate, WORKING), reads only, UNSUBSCRIBE (server only) |
| 20_in_scope | 15 | IN SCOPE placement, isolation of reads / UPDATE / FORGET, refusals (server only) |
| 21_edge_cases | 10 | Boundaries, idempotency, type coercion |
| 22_link_update | 24 | LINK sets the weight, retype by UNLINK + LINK, UNLINK, WEIGHT minimum, both ends |
| 23_graph | 39 | FOLLOW LINKS … DEPTH in both directions, WEIGHT, TYPE ("a", "b"), relationships as LINKs, values as fields |
| 24_group_by | 13 | GROUP BY with aggregates and HAVING |
| 25_window_episodic | 13 | WINDOW on RECALL FROM EPISODIC; WINDOW TOP / SINCE refused |
| 26_update_null | 8 | UPDATE field = null removes the field |
| 27_time | 17 | UPDATE history, AS OF "<time>", WITH HISTORY [BETWEEN], validity as fields |
| 28_merge | 16 | Aliases: read and LINK by alias, links to / from the alias reach the survivor |
| 29_scoped_keys | 11 | Keys per scope (server only) |
| 30_branch | 1 | The BRANCH "b" modifier is refused (server only) |
| 31_namespace_statements | 19 | LINK / UNLINK within a NAMESPACE; a namespace's topics, live queries, UNSUBSCRIBE (server only) |
| 32_merge_namespace | 16 | Aliases within a NAMESPACE (server only) |
| 33_branch_fork_merge | 65 | BRANCH [AS OF], FORK [AS OF], MERGE both ways, conflicts, PREFER SOURCE / TARGET, dropping (server only) |
| 34_use_and_namespaces | 13 | USE in a script, a statement's own NAMESPACE wins, a failing statement stops the script, reads across namespaces (server only) |

## Implementation Notes

### Minimal Implementation

A minimal AQL implementation should pass suites 01-07, 13-17, and 21 (core CRUD + aggregates + validation).

### Full Implementation

A full implementation should pass all 406 tests, against a server (`--url`).

### The WASM playground

`node run-tests.mjs` without `--url` runs the WASM build in `../pkg`, which parses
the v0.5–v0.12 syntax, not v0.13, and skips the suites marked `requires: server`.
Until it is rebuilt on the v0.13 parser it fails most of these suites. It is a
demo; ADB is the reference for conformance.

### Known Gaps

The following require external dependencies and are not tested:
- `LIKE "text" | $embedding … CONFIDENCE t` - requires a vector encoder
- Delivery of subscription events and live-query diffs (SSE / MCP) - the engine's tests

## Spec Reference

Specification: [AQL_SPEC_v0.13.md](../spec/AQL_SPEC_v0.13.md) (§9 lists what changed
from v0.12). The earlier v0.5 base and v0.6–v0.12 additions are in
[`../spec/`](../spec/) for history, listed in [CHANGELOG.md](../CHANGELOG.md).

## Version

- **Language version**: v0.13
- **Release**: 0.13.0 (the test suite is versioned with the language)
- **Total tests**: 406 in 34 suites
- **Bug regressions**: B2–B29 kept where the statement still exists (B10, the SCOPE
  private|shared modifier, was removed with it)
