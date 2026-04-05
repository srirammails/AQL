# AQL v0.5 Test Coverage Matrix

This document maps grammar productions and spec sections to test IDs.

## Coverage Summary

| Dimension | Coverage | Tests | Gaps |
|-----------|----------|-------|------|
| Grammar productions | ~93% | T01-T21 | LIKE $embedding needs vector encoder |
| Semantic validation (§19) | 100% | T17-01 to T17-08 | None |
| Cross-type (stmt x memory) | ~95% | T02-T06, T13 | LOAD only valid on TOOLS by spec |
| Modifier x statement | ~90% | T02, T05, T12-T14 | MIN_CONFIDENCE needs embedding |
| Condition forms | ~95% | T02-07 to T02-13 | Deep nesting (3+ levels) not tested |
| Edge cases / boundaries | ~90% | T21-01 to T21-10 | Concurrent write race needs load test |
| Bug regressions (B1-B29) | 29/29 | All sections | None |
| §16 RTB pipeline | 100% | T12-05 | Embedding path |
| §17 Learning loop | 100% | N/A | Full workflow tested |
| **Overall** | **~92%** | **150 tests** | Embedding-based similarity |

## Statement Coverage

### SCAN (5 tests)

| Test ID | Grammar Production | Spec § |
|---------|-------------------|--------|
| T01-01 | `scan_stmt → WINDOW LAST int` | §5 |
| T01-02 | `scan_stmt → WINDOW LAST duration` | §6 |
| T01-03 | `scan_stmt → WINDOW TOP int BY field` | §6 |
| T01-04 | `scan_stmt → WINDOW SINCE key_expr` | §6 |
| T01-05 | Validation: SCAN only WORKING | §19 |

### RECALL (20 tests)

| Test ID | Grammar Production | Spec § |
|---------|-------------------|--------|
| T02-01 | `condition → field = string` | §6 |
| T02-02 | `condition → field != value` | §6 |
| T02-03 | `condition → field > int` (vs float) | §6 |
| T02-04 | `condition → field < int` (vs float) | §6 |
| T02-05 | `condition → field >= float` | §6 |
| T02-06 | `condition → field <= float` | §6 |
| T02-07 | `conditions → condition AND condition` | §6 |
| T02-08 | `conditions → 3-clause AND` | §6 |
| T02-09 | `conditions → condition OR condition` | §6 |
| T02-10 | `conditions → (condition OR condition)` | §6 |
| T02-11 | `conditions → (A OR B) AND C` | §6 |
| T02-12 | `conditions → A AND (B OR C)` | §6 |
| T02-13 | `conditions → (A OR B) AND (C OR D)` | §6 |
| T02-14 | `modifiers → ORDER BY field ASC` | §14 |
| T02-15 | `modifiers → ORDER BY field DESC LIMIT int` | §14 |
| T02-16 | `recall_stmt → FROM SEMANTIC` | §5 |
| T02-17 | `recall_stmt → FROM PROCEDURAL` | §5 |
| T02-18 | `recall_stmt → FROM WORKING` | §5 |
| T02-19 | `recall_stmt → FROM TOOLS` | §5 |
| T02-20 | `recall_stmt → FROM ALL` | §5 |

### LOOKUP (5 tests)

| Test ID | Grammar Production | Spec § |
|---------|-------------------|--------|
| T03-01 | `lookup_stmt → FROM SEMANTIC KEY` | §5 |
| T03-02 | `lookup_stmt → FROM SEMANTIC WHERE` | §5 |
| T03-03 | `lookup_stmt → FROM PROCEDURAL WHERE` | §5 |
| T03-04 | Validation: LOOKUP invalid on WORKING | §19 |
| T03-05 | Validation: LOOKUP invalid on EPISODIC | §19 |

### LOAD (2 tests)

| Test ID | Grammar Production | Spec § |
|---------|-------------------|--------|
| T04-01 | `load_stmt → FROM TOOLS WHERE` | §5 |
| T04-02 | Validation: LOAD only TOOLS | §19 |

### STORE (8 tests)

| Test ID | Grammar Production | Spec § |
|---------|-------------------|--------|
| T05-01 | `store_stmt → TTL duration` | §7 |
| T05-02 | `store_stmt → SCOPE NAMESPACE` | §7 |
| T05-03 | `store_stmt → INTO SEMANTIC` | §7 |
| T05-04 | `store_stmt → INTO PROCEDURAL` | §7 |
| T05-05 | `store_stmt → INTO TOOLS` | §7 |
| T05-06 | TTL expiry behavior | §14 |
| T05-07 | TTL units (ms/s/m/h/d) | §14 |
| T05-08 | Validation: STORE INTO ALL | §19 |

### UPDATE (10 tests)

| Test ID | Grammar Production | Spec § | Bug |
|---------|-------------------|--------|-----|
| T06-01 | `update_stmt → additive fields` | §7 | |
| T06-02 | `update_stmt → PROCEDURAL confidence` | §7 | |
| T06-03 | PROCEDURAL steps writable | §7 | B24 |
| T06-04 | PROCEDURAL variables writable | §7 | B21 |
| T06-05 | PROCEDURAL success/failure counts | §7 | B25 |
| T06-06 | PROCEDURAL version increment | §7 | B26 |
| T06-07 | UPDATE/TTL race condition | §7 | B22 |
| T06-08 | `update_stmt → INTO SEMANTIC` | §7 | |
| T06-09 | Int literal vs stored float | §6 | B23 |
| T06-10 | UPDATE non-existent record | §7 | |

### FORGET (6 tests)

| Test ID | Grammar Production | Spec § | Bug |
|---------|-------------------|--------|-----|
| T07-01 | `forget_stmt → FROM WORKING WHERE` | §8 | |
| T07-02 | FORGET idempotent | §8 | |
| T07-03 | `forget_stmt → FROM EPISODIC WHERE` | §8 | |
| T07-04 | `forget_stmt → FROM ALL WHERE` | §8 | B15 |
| T07-05 | Validation: predicate required | §8 | |
| T07-06 | Validation: FROM ALL no WHERE | §19 | |

### LINK (6 tests)

| Test ID | Grammar Production | Spec § | Bug |
|---------|-------------------|--------|-----|
| T08-01 | `link_stmt → SEMANTIC TO PROCEDURAL` | §9 | |
| T08-02 | `link_stmt → PROCEDURAL TO EPISODIC` | §9 | |
| T08-03 | `link_stmt → EPISODIC TO EPISODIC` | §9 | |
| T08-04 | LINK nonexistent source | §9 | B16 |
| T08-05 | LINK nonexistent target | §9 | B16 |
| T08-06 | Validation: LINK FROM ALL | §19 | B14 |

### WITH LINKS (2 tests)

| Test ID | Grammar Production | Spec § | Bug |
|---------|-------------------|--------|-----|
| T09-01 | `with_links → WITH LINKS ALL` | §10 | B7 |
| T09-02 | `with_links → WITH LINKS TYPE` | §10 | |

### FOLLOW LINKS (2 tests)

| Test ID | Grammar Production | Spec § | Bug |
|---------|-------------------|--------|-----|
| T10-01 | `follow_links → SEMANTIC→PROCEDURAL` | §10 | B6 |
| T10-02 | `follow_links → PROCEDURAL→EPISODIC` | §10 | B6 |

### REFLECT (4 tests)

| Test ID | Grammar Production | Spec § | Bug |
|---------|-------------------|--------|-----|
| T11-01 | `reflect_stmt → multi-source` | §11 | |
| T11-02 | `reflect_stmt → FROM ALL` | §11 | B2 |
| T11-03 | `reflect_stmt → THEN STORE` | §11 | B8 |
| T11-04 | REFLECT in PIPELINE | §12 | B9 |

### PIPELINE (5 tests)

| Test ID | Grammar Production | Spec § | Bug |
|---------|-------------------|--------|-----|
| T12-01 | `pipeline_stmt → named 2-stage` | §12 | |
| T12-02 | `pipeline_stmt → anonymous` | §12 | B17 |
| T12-03 | Validation: TIMEOUT required | §12 | B13/B18 |
| T12-04 | `pipeline_stmt → {var} binding` | §12 | B29 |
| T12-05 | Full 5-stage RTB pipeline | §16 | |

### AGGREGATE (10 tests)

| Test ID | Grammar Production | Spec § | Bug |
|---------|-------------------|--------|-----|
| T13-01 | `aggregate → COUNT(*)` | §13 | B27 |
| T13-02 | `aggregate → SUM(field)` | §13 | B3 |
| T13-03 | `aggregate → AVG(field)` | §13 | B3 |
| T13-04 | `aggregate → MIN(field)` | §13 | B3 |
| T13-05 | `aggregate → MAX(field)` | §13 | B3 |
| T13-06 | All 5 operators combined | §13 | B3 |
| T13-07 | AGGREGATE on WORKING | §13 | B3 |
| T13-08 | AGGREGATE on SEMANTIC | §13 | B3 |
| T13-09 | AGGREGATE on PROCEDURAL | §13 | B3 |
| T13-10 | AGGREGATE on TOOLS | §13 | B27 |

### HAVING (4 tests)

| Test ID | Grammar Production | Spec § | Bug |
|---------|-------------------|--------|-----|
| T14-01 | `having → alias > value` (pass) | §13 | B28 |
| T14-02 | `having → alias > value` (suppress) | §13 | B28 |
| T14-03 | `having → AVG alias` | §13 | B28 |
| T14-04 | `having → multi-alias AND` | §13 | B28 |

### SCOPE/NAMESPACE (5 tests)

| Test ID | Grammar Production | Spec § | Bug |
|---------|-------------------|--------|-----|
| T15-01 | SCOPE private isolation | §14 | B10 |
| T15-02 | SCOPE shared returns correct | §14 | B10 |
| T15-03 | NAMESPACE wrong | §14 | B11 |
| T15-04 | NAMESPACE correct | §14 | B11 |
| T15-05 | SCOPE + NAMESPACE combined | §14 | |

### Dotted Fields (4 tests)

| Test ID | Grammar Production | Spec § | Bug |
|---------|-------------------|--------|-----|
| T16-01 | `field → metadata.namespace` WHERE | §6 | B20 |
| T16-02 | `field → metadata.scope` WHERE | §6 | B20 |
| T16-03 | `field → metadata.*` RETURN | §6 | B20 |
| T16-04 | `field → metadata.version` RETURN | §6 | B20 |

### Validation Rules (8 tests)

| Test ID | Rule | Spec § |
|---------|------|--------|
| T17-01 | SCAN only WORKING | §19 |
| T17-02 | LOAD only TOOLS | §19 |
| T17-03 | FORGET predicate required | §19 |
| T17-04 | STORE INTO ALL invalid | §19 |
| T17-05 | LINK FROM ALL invalid | §19 |
| T17-06 | LOOKUP invalid WORKING | §19 |
| T17-07 | LOOKUP invalid EPISODIC | §19 |
| T17-08 | PIPELINE TIMEOUT required | §19 |

### Edge Cases (10 tests)

| Test ID | Description |
|---------|-------------|
| T21-01 | Empty RECALL returns 0 records |
| T21-02 | FORGET idempotent on deleted |
| T21-03 | UPDATE nonexistent record |
| T21-04 | RECALL empty links array |
| T21-05 | LIMIT before AGGREGATE |
| T21-06 | ORDER BY + LIMIT |
| T21-07 | Boolean false match |
| T21-08 | Float exact match |
| T21-09 | Zero-value float store |
| T21-10 | TTL 1h unit conversion |

## Bug Regression Coverage

All 29 bugs from the spec are covered:

| Bug | Test ID | Description |
|-----|---------|-------------|
| B2 | T11-02 | REFLECT FROM ALL |
| B3 | T13-02..T13-09 | AGGREGATE operators |
| B6 | T10-01, T10-02 | FOLLOW LINKS traversal |
| B7 | T09-01 | WITH LINKS ALL payload |
| B8 | T11-03 | REFLECT THEN STORE |
| B9 | T11-04 | REFLECT in PIPELINE |
| B10 | T15-01, T15-02 | SCOPE isolation |
| B11 | T15-03, T15-04 | NAMESPACE isolation |
| B13 | T12-03 | PIPELINE TIMEOUT required |
| B14 | T08-06 | LINK FROM ALL error |
| B15 | T07-04 | FORGET FROM ALL WHERE |
| B16 | T08-04, T08-05 | LINK nonexistent source/target |
| B17 | T12-02 | PIPELINE anonymous |
| B18 | T12-03 | PIPELINE TIMEOUT enforcement |
| B20 | T16-01..T16-04 | Dotted field paths |
| B21 | T06-04 | PROCEDURAL variables writable |
| B22 | T06-07 | UPDATE/TTL race condition |
| B23 | T06-09 | Int literal vs stored float |
| B24 | T06-03 | PROCEDURAL steps writable |
| B25 | T06-05 | PROCEDURAL success/failure counts |
| B26 | T06-06 | PROCEDURAL version increment |
| B27 | T13-01, T13-10 | COUNT aggregation |
| B28 | T14-01..T14-04 | HAVING alias resolution |
| B29 | T12-04 | PIPELINE {var} binding |

## Known Gaps

1. **LIKE $embedding** - Requires vector encoder, not testable via text queries
2. **PATTERN $embedding** - Requires vector encoder
3. **Deep nesting** - 3+ levels of parentheses not tested
4. **Concurrent writes** - Race conditions need load testing
