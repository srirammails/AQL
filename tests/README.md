# AQL v0.5 Conformance Test Suite

This directory contains the official conformance test suite for AQL (Agent Query Language) v0.5.

**150 tests** covering all grammar productions, semantic validation rules, and edge cases.

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
│   └── 21_edge_cases.yaml    # Edge case tests (T21-01 to T21-10)
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
| 21_edge_cases | 10 | Boundaries, idempotency, type coercion |

## Implementation Notes

### Minimal Implementation

A minimal AQL implementation should pass suites 01-07, 13-17, and 21 (core CRUD + aggregates + validation).

### Full Implementation

A full implementation should pass all 150 tests including LINK, REFLECT, and PIPELINE.

### Known Gaps

The following require external dependencies and are not tested:
- `LIKE $embedding` - requires vector encoder
- `PATTERN $embedding` - requires vector encoder

## Spec Reference

Full specification: [AQL_SPEC_v0.5.md](../spec/AQL_SPEC_v0.5.md)

## Version

- **Spec Version**: 0.5
- **Test Suite Version**: 1.0
- **Total Tests**: 150
- **Bug Regressions**: 29/29 confirmed fixed
