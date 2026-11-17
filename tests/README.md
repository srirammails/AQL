# AQL v0.12 Conformance Test Suite

This directory contains the official conformance test suite for AQL (Agent Query
Language) v0.12, release 0.12.0: the v0.5 base and every version since.

**321 tests in 31 suites**: 116 for the v0.5 core, 177 for v0.6–v0.10, 6 for v0.11
and 22 for v0.12. ADB 0.12.0 passes all 321.

## Purpose

Any AQL implementation (Rust, Python, TypeScript, Java, etc.) can run this test suite to verify correctness against the specification.

## Structure

```
tests/
├── README.md                 # This file
├── fixtures/
│   └── seed.aql              # Canonical seed data - run BEFORE any test suite
├── suites/
│   ├── 01_scan.yaml          # SCAN tests (T01-01 to T01-05)
│   ├── 02_recall.yaml        # RECALL tests (T02-01 to T02-20)
│   ├── 03_lookup.yaml        # LOOKUP tests (T03-01 to T03-05)
│   ├── 04_load.yaml          # LOAD tests (T04-01 to T04-02)
│   ├── 05_store.yaml         # STORE tests (T05-01 to T05-08)
│   ├── 06_update.yaml        # UPDATE tests (T06-01 to T06-10)
│   ├── 07_forget.yaml        # FORGET tests (T07-01 to T07-06)
│   ├── 08_link.yaml          # LINK tests (T08-01 to T08-06)
│   ├── 09_with_links.yaml    # WITH LINKS tests (T09-01 to T09-02)
│   ├── 10_follow_links.yaml  # FOLLOW LINKS tests (T10-01 to T10-02)
│   ├── 11_reflect.yaml       # REFLECT tests (T11-01 to T11-04)
│   ├── 12_pipeline.yaml      # PIPELINE tests (T12-01 to T12-05)
│   ├── 13_aggregate.yaml     # AGGREGATE tests (T13-01 to T13-10)
│   ├── 14_having.yaml        # HAVING tests (T14-01 to T14-04)
│   ├── 15_scope_namespace.yaml # SCOPE/NAMESPACE tests (T15-01 to T15-05)
│   ├── 16_dotted_fields.yaml # Dotted field path tests (T16-01 to T16-04)
│   ├── 17_validation.yaml    # §19 validation rule tests (T17-01 to T17-08)
│   ├── 18_subscriptions.yaml … 20_in_scope.yaml  # v0.6 (server only)
│   ├── 21_edge_cases.yaml    # Edge case tests (T21-01 to T21-10)
│   ├── 22_link_update.yaml   # v0.7 UPDATE LINK / UNLINK / re-LINK (T22-01 to T22-24)
│   ├── 23_graph.yaml         # v0.8 graph reads and facts
│   ├── 24_group_by.yaml … 26_update_null.yaml    # v0.9
│   ├── 27_time.yaml … 29_scoped_keys.yaml        # v0.10
│   ├── 30_branch.yaml        # v0.11 BRANCH (server only)
│   └── 31_namespace_statements.yaml               # v0.12 NAMESPACE on links and topics (server only)
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
watch id, an offset) for later queries and `fields` as `${name}`. A suite with
`requires: server` uses engine-only statements (v0.6) and is skipped unless the
runner targets a server (`node run-tests.mjs --url http://host:port`).

## Test Categories

| Suite | Tests | Coverage |
|-------|-------|----------|
| 01_scan | 5 | WINDOW LAST N, duration, TOP BY, SINCE |
| 02_recall | 20 | All comparators, AND/OR, ORDER BY, LIMIT, FROM ALL |
| 03_lookup | 5 | KEY, WHERE, validation errors |
| 04_load | 2 | TOOLS ranking, validation |
| 05_store | 8 | All memory types, TTL, SCOPE, NAMESPACE |
| 06_update | 10 | Additive fields, version increment, TTL race |
| 07_forget | 6 | All types, idempotency, FROM ALL WHERE |
| 08_link | 6 | Cross-type edges, error handling |
| 09_with_links | 2 | Links payload in responses |
| 10_follow_links | 2 | Single-hop traversal |
| 11_reflect | 4 | Multi-source, FROM ALL, THEN clause |
| 12_pipeline | 5 | Named/anonymous, TIMEOUT, {var} binding |
| 13_aggregate | 10 | COUNT, SUM, AVG, MIN, MAX on all types |
| 14_having | 4 | Alias resolution, suppress, multi-alias |
| 15_scope_namespace | 5 | Isolation tests |
| 16_dotted_fields | 4 | metadata.namespace, metadata.scope, metadata.version |
| 17_validation | 8 | All §19 semantic rules |
| 18_subscriptions | 14 | v0.6: SUBSCRIBE (TOPIC / SCOPE, KINDS, WHERE, FROM), PUBLISH, ACK, UNSUBSCRIBE (server only) |
| 19_watch | 13 | v0.6: WATCH snapshots (RECALL / LOOKUP / SCAN, ORDER BY, aggregate), DEBOUNCE range, read-only, UNWATCH (server only) |
| 20_in_scope | 15 | v0.6: IN SCOPE placement, isolation of reads / UPDATE / FORGET, with SCOPE shared, refusals (server only) |
| 21_edge_cases | 10 | Boundaries, idempotency, type coercion |
| 22_link_update | 24 | v0.7: UPDATE LINK (weight / type), UNLINK, re-LINK updates the weight |
| 23_graph | 34 | v0.8: FOLLOW LINKS … DEPTH, direction, MIN_WEIGHT, TYPE IN, STORE … FACT |
| 24_group_by | 13 | v0.9: GROUP BY with aggregates and HAVING |
| 25_window_episodic | 13 | v0.9: WINDOW on RECALL FROM EPISODIC |
| 26_update_null | 8 | v0.9: UPDATE field = null removes the field |
| 27_time | 18 | v0.10: fact validity, AS OF, VALID AT, WITH HISTORY |
| 28_merge | 13 | v0.10: MERGE ENTITY moves facts and links; the key becomes an alias |
| 29_scoped_keys | 12 | v0.10: keys per scope (server only) |
| 30_branch | 6 | v0.11: BRANCH "main", unknown branches, BRANCH needs NAMESPACE (server only) |
| 31_namespace_statements | 22 | v0.12: LINK / UPDATE LINK / UNLINK within a NAMESPACE; a namespace's topics, ACK and UNSUBSCRIBE (server only) |

## Implementation Notes

### Minimal Implementation

A minimal AQL implementation should pass suites 01-07, 13-17, and 21 (core CRUD + aggregates + validation).

### Full Implementation

A full implementation should pass all 321 tests, against a server (`--url`).

### The WASM playground

`node run-tests.mjs` without `--url` runs the WASM build in `../pkg`, which parses
v0.12 but executes the v0.5 core only, and skips the suites marked `requires: server`.
The suites have been tightened since the playground was written: it passes 84 of the
116 v0.5 core tests (and 115 of the 239 it runs). It is a demo; ADB is the reference
for conformance.

### Known Gaps

The following require external dependencies and are not tested:
- `LIKE $embedding` - requires vector encoder
- `PATTERN $embedding` - requires vector encoder

## Spec Reference

Base specification: [AQL_SPEC_v0.5.md](../spec/AQL_SPEC_v0.5.md); each later
version's additions are in [`../spec/`](../spec/), listed in [CHANGELOG.md](../CHANGELOG.md).

## Version

- **Language version**: v0.12
- **Release**: 0.12.0 (the test suite is versioned with the language)
- **Total tests**: 321 in 31 suites
- **Bug regressions**: 29/29 confirmed fixed
