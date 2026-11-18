# AQL v0.13 Test Coverage Matrix

This document maps the v0.13 statements and modifiers (spec/AQL_SPEC_v0.13.md) to
test IDs. The suites were rewritten from the v0.5–v0.12 syntax; each suite's
header lists what changed and which tests were removed for lack of a v0.13
equivalent.

## Coverage Summary

| Dimension | Coverage | Tests | Gaps |
|-----------|----------|-------|------|
| RECALL (§1) on every memory type | WORKING, EPISODIC, SEMANTIC, PROCEDURAL, TOOLS, ALL | suites 01–04, 02, 11 | `LIKE` needs a vector encoder |
| STORE / UPDATE / FORGET (§2) | all types, TTL, null fields, history | 05–07, 26, 27 | FORGET without WHERE (would erase the shared seed) |
| LINK / UNLINK (§3) | create or update, TYPE, WEIGHT, both directions | 08, 22, 23, 31 | — |
| Aliases (§4) | read / LINK by alias, links to and from the alias, per namespace | 28, 32 | — |
| BRANCH / FORK / MERGE (§5) | AS OF, own writes, independence, both merge directions, conflicts, PREFER, drop | 33 | MERGE of a fork INTO its base only with PREFER |
| PIPELINE (§6) | named / anonymous, TIMEOUT, {var}, write last | 11, 12 | — |
| SUBSCRIBE / UNSUBSCRIBE / PUBLISH (§7) | topics, live queries, NAMESPACE | 18, 19, 31 | Deliveries over SSE / MCP (engine tests) |
| Modifiers (§8) | NAMESPACE, IN SCOPE, LIMIT, ORDER BY, RETURN, TTL, AGGREGATE, GROUP BY, HAVING, WINDOW, WITH / FOLLOW LINKS, AS OF, WITH HISTORY, TIMEOUT | throughout | `CONFIDENCE` (needs embeddings) |
| Conditions (§8) | = != < <= > >=, BETWEEN, IN (…), STARTS_WITH, AND, OR, ( ) | 01, 02, 20, 27, 33 | CONTAINS, ENDS_WITH not tested |
| Replaced v0.5–v0.12 forms (§9) refused | LOOKUP, LOAD, SCAN, KEY, LINK … TO, PATTERN … THRESHOLD, SCOPE private, WATCH, REFLECT, TYPE IN, INBOUND, MIN_WEIGHT, FACT, WINDOW TOP / SINCE, AS OF … AGO, VALID AT, MERGE ENTITY, BRANCH "b" | 17, 23, 25, 27, 28, 30 | ACK, DEBOUNCE, FROM NOW, KINDS (removed tests, not refusal tests) |
| Bug regressions | B2–B29 where the statement still exists | see below | B10 (SCOPE modifier removed) |
| **Overall** | | **406 tests in 34 suites** (246 + 160 server only) | |

## Tests per suite

| Suite | Tests | From v0.12 | Server only |
|---|---|---|---|
| 01_scan | 5 | 5 translated | |
| 02_recall | 20 | unchanged | |
| 03_lookup | 4 | 4 translated, 1 removed | |
| 04_load | 2 | 1 translated, 1 removed, 1 added | |
| 05_store | 8 | 4 unchanged, 4 translated | |
| 06_update | 10 | unchanged | |
| 07_forget | 4 | 2 unchanged, 2 translated, 2 removed | |
| 08_link | 6 | 6 translated | |
| 09_with_links | 2 | 2 translated | |
| 10_follow_links | 2 | unchanged | |
| 11_reflect | 4 | 4 translated | |
| 12_pipeline | 6 | 2 unchanged, 3 translated, 1 added | |
| 13_aggregate | 10 | unchanged | |
| 14_having | 4 | unchanged | |
| 15_scope_namespace | 3 | 2 unchanged, 1 translated, 2 removed | |
| 16_dotted_fields | 3 | 2 unchanged, 1 translated, 1 removed | |
| 17_validation | 13 | 2 unchanged, 5 translated, 1 removed, 6 added | |
| 18_subscriptions | 9 | 5 unchanged, 4 translated, 5 removed | yes |
| 19_watch | 11 | 2 unchanged, 9 translated, 2 removed | yes |
| 20_in_scope | 15 | 8 unchanged, 7 translated | yes |
| 21_edge_cases | 10 | 9 unchanged, 1 translated | |
| 22_link_update | 24 | 5 unchanged, 16 translated, 3 removed, 3 added | |
| 23_graph | 39 | 7 unchanged, 24 translated, 3 removed, 8 added | |
| 24_group_by | 13 | unchanged | |
| 25_window_episodic | 13 | 9 unchanged, 2 translated, 2 removed, 2 added | |
| 26_update_null | 8 | unchanged | |
| 27_time | 17 | 14 translated, 4 removed, 3 added | |
| 28_merge | 16 | 10 translated, 3 removed, 6 added | |
| 29_scoped_keys | 11 | 6 unchanged, 5 translated, 1 removed | yes |
| 30_branch | 1 | 6 removed, 1 added | yes |
| 31_namespace_statements | 19 | 7 unchanged, 10 translated, 5 removed, 2 added | yes |
| 32_merge_namespace | 16 | 10 translated, 3 removed, 6 added | yes |
| 33_branch_fork_merge | 65 | new | yes |
| 34_use_and_namespaces | 13 | new: USE, scripts, NAMESPACE ("a", "b") | 13 |
| **Total** | **406** | 334 before: 45 removed, 117 added | 160 |

## Bug Regression Coverage

| Bug | Test ID | Description |
|-----|---------|-------------|
| B2 | T11-02 | RECALL FROM ALL (was REFLECT FROM ALL) |
| B3 | T13-02..T13-09 | AGGREGATE operators |
| B6 | T10-01, T10-02 | FOLLOW LINKS traversal |
| B7 | T09-01 | WITH LINKS payload |
| B8 | T11-03 | PIPELINE with a STORE last (was REFLECT THEN STORE) |
| B9 | T11-04 | PIPELINE with an UPDATE last (was REFLECT in PIPELINE) |
| B10 | — | SCOPE private / shared isolation: removed with the SCOPE modifier |
| B11 | T15-01, T15-02 | NAMESPACE isolation |
| B13 | T12-03 | PIPELINE TIMEOUT required |
| B14 | T08-06 | LINK FROM ALL error |
| B15 | T07-04 | FORGET FROM ALL WHERE |
| B16 | T08-04, T08-05, T22-12 | LINK nonexistent source/target |
| B17 | T12-02 | PIPELINE anonymous |
| B18 | T12-03 | PIPELINE TIMEOUT enforcement |
| B20 | T16-01..T16-03 | Dotted field paths |
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

1. **LIKE … CONFIDENCE** - requires a vector encoder, not testable via text queries
2. **FORGET without WHERE** on a memory type - legal in v0.13, not run (it would erase the seed the other suites share); dropping a namespace is tested in 33
3. **CONTAINS / ENDS_WITH** conditions - not tested
4. **Deep nesting** - 3+ levels of parentheses not tested
5. **Concurrent writes** - race conditions need load testing

## Statement Coverage by Test

### 01_scan — RECALL_WORKING (5 tests)

| Test ID | Checks | Expect |
|---|---|---|
| T01-01 | RECALL FROM WORKING WINDOW LAST N | ok |
| T01-02 | RECALL FROM WORKING WINDOW LAST duration | ok |
| T01-03 | ORDER BY field DESC LIMIT 1 (was WINDOW TOP 1 BY field) | ok |
| T01-04 | WHERE created_at BETWEEN … AND … (was WINDOW SINCE) | ok |
| T01-05 | WINDOW is valid on EPISODIC too (was: SCAN only on WORKING) | ok |

### 02_recall — RECALL (20 tests)

| Test ID | Checks | Expect |
|---|---|---|
| T02-01 | WHERE = string | ok |
| T02-02 | Comparator != | ok |
| T02-03 | Comparator > int vs float | ok |
| T02-04 | Comparator < int vs float | ok |
| T02-05 | Comparator >= float | ok |
| T02-06 | Comparator <= float | ok |
| T02-07 | AND condition | ok |
| T02-08 | 3-clause AND | ok |
| T02-09 | OR bare | ok |
| T02-10 | (A OR B) parenthesised | ok |
| T02-11 | (A OR B) AND C | ok |
| T02-12 | A AND (B OR C) | ok |
| T02-13 | (A OR B) AND (C OR D) | ok |
| T02-14 | ORDER BY ASC | ok |
| T02-15 | ORDER BY DESC LIMIT | ok |
| T02-16 | RECALL SEMANTIC | ok |
| T02-17 | RECALL PROCEDURAL | ok |
| T02-18 | RECALL WORKING | ok |
| T02-19 | RECALL TOOLS | ok |
| T02-20 | RECALL FROM ALL | ok |

### 03_lookup — RECALL_KEY (4 tests)

| Test ID | Checks | Expect |
|---|---|---|
| T03-01 | RECALL SEMANTIC WHERE concept = (was LOOKUP KEY) | ok |
| T03-02 | RECALL SEMANTIC WHERE comparison | ok |
| T03-03 | RECALL PROCEDURAL WHERE | ok |
| T03-04 | an exact read on WORKING (was: LOOKUP invalid on WORKING) | ok |

### 04_load — RECALL_TOOLS (2 tests)

| Test ID | Checks | Expect |
|---|---|---|
| T04-01 | RECALL TOOLS ranking (was LOAD) | ok |
| T04-02 | RECALL TOOLS WHERE task = … LIMIT n | ok |

### 05_store — STORE (8 tests)

| Test ID | Checks | Expect |
|---|---|---|
| T05-01 | STORE WORKING TTL | ok |
| T05-02 | STORE EPISODIC NAMESPACE | ok |
| T05-03 | STORE SEMANTIC | ok |
| T05-04 | STORE PROCEDURAL | ok |
| T05-05 | STORE TOOLS | ok |
| T05-06 | TTL 500ms expiry | ok |
| T05-07 | TTL units ms/s/m/h/d | ok |
| T05-08 | STORE INTO ALL invalid | refused |

### 06_update — UPDATE (10 tests)

| Test ID | Checks | Expect |
|---|---|---|
| T06-01 | UPDATE EPISODIC additive | ok |
| T06-02 | UPDATE PROCEDURAL confidence | ok |
| T06-03 | PROCEDURAL steps writable (B24) | ok |
| T06-04 | PROCEDURAL variables writable (B21) | ok |
| T06-05 | PROCEDURAL success/failure counts (B25) | ok |
| T06-06 | PROCEDURAL version increment (B26) | ok |
| T06-07 | UPDATE/TTL race condition (B22) | ok |
| T06-08 | UPDATE SEMANTIC | ok |
| T06-09 | Int literal vs stored float (B23) | ok |
| T06-10 | UPDATE non-existent | ok |

### 07_forget — FORGET (4 tests)

| Test ID | Checks | Expect |
|---|---|---|
| T07-01 | FORGET WORKING WHERE | ok |
| T07-02 | FORGET idempotent | ok |
| T07-03 | FORGET EPISODIC WHERE | ok |
| T07-04 | FORGET FROM ALL WHERE (B15) | ok |

### 08_link — LINK (6 tests)

| Test ID | Checks | Expect |
|---|---|---|
| T08-01 | LINK SEMANTIC to PROCEDURAL | ok |
| T08-02 | LINK PROCEDURAL to EPISODIC | ok |
| T08-03 | LINK EPISODIC to EPISODIC same-type | ok |
| T08-04 | LINK nonexistent source (B16) | refused |
| T08-05 | LINK nonexistent target (B16) | refused |
| T08-06 | LINK FROM ALL error (B14) | refused |

### 09_with_links — WITH_LINKS (2 tests)

| Test ID | Checks | Expect |
|---|---|---|
| T09-01 | WITH LINKS, every type (B7) | ok |
| T09-02 | WITH LINKS TYPE filter | ok |

### 10_follow_links — FOLLOW_LINKS (2 tests)

| Test ID | Checks | Expect |
|---|---|---|
| T10-01 | FOLLOW LINKS SEMANTIC to PROC (B6) | ok |
| T10-02 | FOLLOW LINKS PROC to EPISODIC (B6) | ok |

### 11_reflect — PIPELINE_REFLECT (4 tests)

| Test ID | Checks | Expect |
|---|---|---|
| T11-01 | multi-source read as a PIPELINE of RECALLs | ok |
| T11-02 | RECALL FROM ALL (B2, was REFLECT FROM ALL) | ok |
| T11-03 | PIPELINE with a STORE last (B8, was REFLECT THEN STORE) | ok |
| T11-04 | PIPELINE of reads with an UPDATE last (B9, was REFLECT in PIPELINE) | ok |

### 12_pipeline — PIPELINE (6 tests)

| Test ID | Checks | Expect |
|---|---|---|
| T12-01 | PIPELINE named 2-stage | ok |
| T12-02 | PIPELINE anonymous (B17) | ok |
| T12-03 | PIPELINE TIMEOUT required (B13/B18) | refused |
| T12-04 | PIPELINE {var} binding (B29) | ok |
| T12-05 | RTB full 5-stage pipeline | ok |
| T12-06 | TIMEOUT is only valid on PIPELINE | refused |

### 13_aggregate — AGGREGATE (10 tests)

| Test ID | Checks | Expect |
|---|---|---|
| T13-01 | COUNT EPISODIC (B27) | ok |
| T13-02 | SUM EPISODIC (B3) | ok |
| T13-03 | AVG EPISODIC (B3) | ok |
| T13-04 | MIN EPISODIC (B3) | ok |
| T13-05 | MAX EPISODIC (B3) | ok |
| T13-06 | All 5 aggregate operators | ok |
| T13-07 | COUNT+AVG WORKING (B3) | ok |
| T13-08 | COUNT+AVG SEMANTIC (B3) | ok |
| T13-09 | COUNT PROCEDURAL (B3) | ok |
| T13-10 | COUNT TOOLS (B27) | ok |

### 14_having — HAVING (4 tests)

| Test ID | Checks | Expect |
|---|---|---|
| T14-01 | HAVING alias pass (B28) | ok |
| T14-02 | HAVING alias suppress (B28) | ok |
| T14-03 | HAVING AVG alias (B28) | ok |
| T14-04 | HAVING multi-alias AND (B28) | ok |

### 15_scope_namespace — NAMESPACE (3 tests)

| Test ID | Checks | Expect |
|---|---|---|
| T15-01 | NAMESPACE wrong (B11) | ok |
| T15-02 | NAMESPACE correct (B11) | ok |
| T15-03 | NAMESPACE returns that namespace's records | ok |

### 16_dotted_fields — DOTTED_FIELDS (3 tests)

| Test ID | Checks | Expect |
|---|---|---|
| T16-01 | WHERE metadata.namespace (B20) | ok |
| T16-02 | RETURN metadata.namespace (B20) | ok |
| T16-03 | RETURN metadata.version (B20) | ok |

### 17_validation — VALIDATION (12 tests)

| Test ID | Checks | Expect |
|---|---|---|
| T17-01 | SCAN is not a v0.13 verb | refused |
| T17-02 | LOAD is not a v0.13 verb | refused |
| T17-03 | LOOKUP is not a v0.13 verb | refused |
| T17-04 | KEY x = v is refused (WHERE x = v) | refused |
| T17-05 | STORE INTO ALL invalid | refused |
| T17-06 | LINK FROM ALL invalid | refused |
| T17-07 | PIPELINE TIMEOUT required | refused |
| T17-08 | LINK … TO … is refused (LINK … INTO …) | refused |
| T17-09 | PATTERN … THRESHOLD is refused (LIKE … CONFIDENCE) | refused |
| T17-10 | the SCOPE private modifier is refused | refused |
| T17-11 | WATCH is refused (SUBSCRIBE RECALL …) | refused |
| T17-12 | REFLECT is refused (PIPELINE) | refused |

### 18_subscriptions — SUBSCRIPTIONS (9 tests, server only)

| Test ID | Checks | Expect |
|---|---|---|
| T18-01 | SUBSCRIBE TOPIC returns a handle | ok |
| T18-02 | PUBLISH writes an event and returns its offset | ok |
| T18-03 | SUBSCRIBE TOPIC with WHERE | ok |
| T18-04 | SUBSCRIBE to a second topic | ok |
| T18-05 | PUBLISH needs a payload | refused |
| T18-06 | PUBLISH to a second topic | ok |
| T18-07 | UNSUBSCRIBE stops the subscription | ok |
| T18-08 | UNSUBSCRIBE again: nothing to stop | ok |
| T18-09 | keywords are case-insensitive | ok |

### 19_watch — LIVE_QUERIES (11 tests, server only)

| Test ID | Checks | Expect |
|---|---|---|
| T19-01 | setup: a scoped record | ok |
| T19-02 | setup: a second scoped record | ok |
| T19-03 | SUBSCRIBE RECALL returns the snapshot | ok |
| T19-04 | SUBSCRIBE RECALL by key | ok |
| T19-05 | SUBSCRIBE RECALL with ORDER BY and LIMIT | ok |
| T19-06 | SUBSCRIBE to an aggregate | ok |
| T19-07 | SUBSCRIBE RECALL FROM WORKING WINDOW (was WATCH SCAN) | ok |
| T19-08 | only reads are live queries: a write is a parse error | refused |
| T19-09 | a PIPELINE can't be subscribed to | refused |
| T19-10 | UNSUBSCRIBE stops the live query | ok |
| T19-11 | UNSUBSCRIBE again: nothing to stop | ok |

### 20_in_scope — IN_SCOPE (15 tests, server only)

| Test ID | Checks | Expect |
|---|---|---|
| T20-01 | STORE in agent:conf-1 | ok |
| T20-02 | STORE in agent:conf-2 | ok |
| T20-03 | IN SCOPE after the memory type, no WHERE | ok |
| T20-04 | IN SCOPE among the modifiers | ok |
| T20-05 | IN membership inside WHERE, then IN SCOPE | ok |
| T20-06 | a read by key in another scope finds nothing | ok |
| T20-07 | UPDATE in another scope changes nothing | ok |
| T20-08 | UPDATE in its own scope | ok |
| T20-09 | FORGET in another scope forgets nothing | ok |
| T20-10 | the other scope's record is still there | ok |
| T20-11 | STORE in a team scope | ok |
| T20-12 | RECALL IN SCOPE the team scope | ok |
| T20-13 | EPISODIC in a named scope | ok |
| T20-14 | IN SCOPE on WORKING is refused (long-term memory types only) | refused |
| T20-15 | a malformed scope name is refused | refused |

### 21_edge_cases — EDGE_CASES (10 tests)

| Test ID | Checks | Expect |
|---|---|---|
| T21-01 | Empty RECALL returns 0 records | ok |
| T21-02 | FORGET idempotent on deleted record | ok |
| T21-03 | UPDATE nonexistent record | ok |
| T21-04 | RECALL empty links array | ok |
| T21-05 | LIMIT before AGGREGATE | ok |
| T21-06 | ORDER BY + LIMIT | ok |
| T21-07 | Boolean false match | ok |
| T21-08 | Float exact match | ok |
| T21-09 | Zero-value float store | ok |
| T21-10 | TTL 1h unit conversion | ok |

### 22_link_update — LINK_UPDATE (24 tests)

| Test ID | Checks | Expect |
|---|---|---|
| T22-01 | setup: record a | ok |
| T22-02 | setup: record b | ok |
| T22-03 | LINK a → b | ok |
| T22-04 | LINK again, same type: updates the weight | ok |
| T22-05 | one link, weight 0.7 | ok |
| T22-06 | LINK … WEIGHT sets the weight (was UPDATE LINK … WEIGHT) | ok |
| T22-07 | the weight changed | ok |
| T22-08 | retype, step 1: UNLINK the old type | ok |
| T22-09 | retype, step 2: LINK the new type (was UPDATE LINK (type = …, weight = …)) | ok |
| T22-10 | FOLLOW LINKS the new type reaches b | ok |
| T22-11 | the old type is gone | ok |
| T22-12 | LINK with no matching source record fails | refused |
| T22-13 | a second link type between the same records | ok |
| T22-14 | UNLINK … TYPE removes only that type | ok |
| T22-15 | the other link stays | ok |
| T22-16 | UNLINK without TYPE removes every link between them | ok |
| T22-17 | no links left; the records stay | ok |
| T22-18 | UNLINK again is a no-op | ok |
| T22-19 | setup: record c | ok |
| T22-20 | LINK a → c, weight 0.3 | ok |
| T22-21 | LINK again without WEIGHT keeps the weight | ok |
| T22-22 | still one link, weight 0.3 | ok |
| T22-23 | WITH LINKS WEIGHT is a minimum: 0.3 < 0.5 | ok |
| T22-24 | the link is read from its other end too (both directions) | ok |

### 23_graph — GRAPH (39 tests)

| Test ID | Checks | Expect |
|---|---|---|
| T23-01 | setup: record g_a | ok |
| T23-02 | setup: record g_b | ok |
| T23-03 | setup: record g_c | ok |
| T23-04 | setup: record g_d | ok |
| T23-05 | setup: record g_x | ok |
| T23-06 | setup: LINK g_a → g_b (next) | ok |
| T23-07 | setup: LINK g_b → g_c (next) | ok |
| T23-08 | setup: LINK g_c → g_d (next) | ok |
| T23-09 | setup: LINK g_x → g_b (next) | ok |
| T23-10 | setup: LINK g_a → g_x (knows) | ok |
| T23-11 | DEPTH 1 (default): one hop | ok |
| T23-12 | DEPTH 3: every record within 3 hops, nearest first, by path weight within a hop | ok |
| T23-13 | cycle-safe: g_b and g_x are each returned once | ok |
| T23-14 | both directions: every record linked to g_b, by link weight | ok |
| T23-15 | WEIGHT drops light links (at least 0.5) | ok |
| T23-16 | TYPE ("a", "b") picks several types | ok |
| T23-17 | no type: every type | ok |
| T23-18 | RECALL by key … FOLLOW LINKS returns the linked records | ok |
| T23-19 | WITH LINKS lists the links at either end of the record | ok |
| T23-20 | WITH LINKS TYPE (…) WEIGHT | ok |
| T23-21 | TYPE IN (…) is refused (TYPE (…)) | refused |
| T23-22 | INBOUND is refused (links are followed both ways) | refused |
| T23-23 | MIN_WEIGHT is refused (WEIGHT) | refused |
| T23-24 | setup: record gf:ada | ok |
| T23-25 | setup: record gf:acme | ok |
| T23-26 | setup: record gf:globex | ok |
| T23-27 | setup: record gf:bob | ok |
| T23-28 | setup: record gf:cy | ok |
| T23-29 | Ada works at Acme: a LINK of TYPE works_at (was STORE … FACT) | ok |
| T23-30 | the relationship is followed as a link | ok |
| T23-31 | a change of employer, step 1: UNLINK the old one | ok |
| T23-32 | a change of employer, step 2: LINK the new one (was FACT superseding) | ok |
| T23-33 | one employer now | ok |
| T23-34 | several objects: LINK knows Bob (was FACT MULTI) | ok |
| T23-35 | several objects: LINK knows Cy | ok |
| T23-36 | both are followed | ok |
| T23-37 | a value is a field (was FACT … value =) | ok |
| T23-38 | the field is on the record; it is not a link | ok |
| T23-39 | STORE … FACT is refused | refused |

### 24_group_by — GROUP_BY (13 tests)

| Test ID | Checks | Expect |
|---|---|---|
| T24-01 | setup: visit 1 | ok |
| T24-02 | setup: visit 2 | ok |
| T24-03 | setup: visit 3 | ok |
| T24-04 | setup: visit 4 | ok |
| T24-05 | setup: visit 5 | ok |
| T24-06 | count and sum per url; null group last ascending | ok |
| T24-07 | HAVING filters groups | ok |
| T24-08 | ORDER BY an aggregate, LIMIT and RETURN on groups | ok |
| T24-09 | HAVING that no group passes gives no rows | ok |
| T24-10 | several aggregates per group | ok |
| T24-11 | AGGREGATE may come before GROUP BY | ok |
| T24-12 | GROUP BY requires AGGREGATE | refused |
| T24-13 | HAVING requires AGGREGATE | refused |

### 25_window_episodic — WINDOW_EPISODIC (13 tests)

| Test ID | Checks | Expect |
|---|---|---|
| T25-01 | setup: episode 1 | ok |
| T25-02 | setup: episode 2 | ok |
| T25-03 | setup: episode 3 | ok |
| T25-04 | setup: episode 4 | ok |
| T25-05 | setup: episode 5 | ok |
| T25-06 | WINDOW LAST n keeps the newest n | ok |
| T25-07 | WINDOW LAST duration | ok |
| T25-08 | a comparison in WHERE (was WINDOW SINCE n = 3) | ok |
| T25-09 | ORDER BY f DESC LIMIT n (was WINDOW TOP n BY f) | ok |
| T25-10 | window first, then ORDER BY and LIMIT | ok |
| T25-11 | window before AGGREGATE | ok |
| T25-12 | WINDOW TOP … BY is refused | refused |
| T25-13 | WINDOW SINCE is refused | refused |

### 26_update_null — UPDATE_NULL (8 tests)

| Test ID | Checks | Expect |
|---|---|---|
| T26-01 | setup: record with a note | ok |
| T26-02 | UPDATE SEMANTIC with note = null | ok |
| T26-03 | the note is gone, the other field changed | ok |
| T26-04 | a removed field no longer matches | ok |
| T26-05 | setup: episode with a note | ok |
| T26-06 | UPDATE EPISODIC with note = null | ok |
| T26-07 | the episode's note is gone | ok |
| T26-08 | the episode itself stays | ok |

### 27_time — TIME (17 tests)

| Test ID | Checks | Expect |
|---|---|---|
| T27-01 | setup: Ada works at Acme (a field) | ok |
| T27-02 | the first version's time | ok |
| T27-03 | UPDATE keeps the old version (was: FACT superseding) | ok |
| T27-04 | current view: the new value | ok |
| T27-05 | WITH HISTORY keeps the replaced version | ok |
| T27-06 | AS OF the first version's time: the record as it was | ok |
| T27-07 | AS OF before the run: nothing | ok |
| T27-08 | WITH HISTORY BETWEEN a wide range: both versions | ok |
| T27-09 | WITH HISTORY BETWEEN a range before the run: none | ok |
| T27-10 | WITH HISTORY counts versions | ok |
| T27-11 | setup: validity as ordinary fields, a past role | ok |
| T27-12 | setup: the current role | ok |
| T27-13 | the role at a date is a WHERE on the fields (was VALID AT) | ok |
| T27-14 | AS OF … AGO is refused (a timestamp or a variable) | refused |
| T27-15 | VALID AT is refused | refused |
| T27-16 | no history reads on WORKING | refused |
| T27-17 | FORGET removes the records | ok |

### 28_merge — ALIASES (16 tests)

| Test ID | Checks | Expect |
|---|---|---|
| T28-01 | setup: the survivor | ok |
| T28-02 | setup: the duplicate | ok |
| T28-03 | setup: notes | ok |
| T28-04 | setup: bob | ok |
| T28-05 | setup: a link from the duplicate | ok |
| T28-06 | setup: a link to the duplicate | ok |
| T28-07 | the duplicate's key becomes an alias of the survivor (was MERGE ENTITY) | ok |
| T28-08 | FORGET the duplicate's own record | ok |
| T28-09 | a read by the alias finds the survivor | ok |
| T28-10 | the duplicate's record is gone | ok |
| T28-11 | a link from the alias reaches the survivor | ok |
| T28-12 | a link to the alias reaches the survivor | ok |
| T28-13 | a LINK naming the alias | ok |
| T28-14 | ... is the survivor's link | ok |
| T28-15 | the survivor carries every link that names it or its alias | ok |
| T28-16 | MERGE ENTITY is refused | refused |

### 29_scoped_keys — SCOPED_KEYS (11 tests, server only)

| Test ID | Checks | Expect |
|---|---|---|
| T29-01 | setup: a key in the default scope | ok |
| T29-02 | the same key in a named scope | ok |
| T29-03 | and in another | ok |
| T29-04 | a key is unique within its scope | refused |
| T29-05 | WHERE key = with IN SCOPE takes the bare key | ok |
| T29-06 | without IN SCOPE the key matches in every scope | ok |
| T29-07 | a value in one scope (was a FACT IN SCOPE) | ok |
| T29-08 | the same field in another scope doesn't replace it | ok |
| T29-09 | each scope has its value | ok |
| T29-10 | both are current | ok |
| T29-11 | a key can't contain ~ | refused |

### 30_branch — BRANCH_MODIFIER_REMOVED (1 tests, server only)

| Test ID | Checks | Expect |
|---|---|---|
| T30-01 | the BRANCH "b" modifier is refused (use NAMESPACE "<branch>") | refused |

### 31_namespace_statements — NAMESPACE_STATEMENTS (19 tests, server only)

| Test ID | Checks | Expect |
|---|---|---|
| T31-01 | setup: a in namespace p | ok |
| T31-02 | setup: b in namespace p | ok |
| T31-03 | setup: a in namespace q | ok |
| T31-04 | setup: b in namespace q | ok |
| T31-05 | LINK … NAMESPACE links p's records only | ok |
| T31-06 | p's record has the link | ok |
| T31-07 | q's record has none | ok |
| T31-08 | LINK … NAMESPACE p again sets the weight | ok |
| T31-09 | the weight moved | ok |
| T31-10 | a LINK in p doesn't reach q | ok |
| T31-11 | UNLINK … NAMESPACE p | ok |
| T31-12 | LINK naming a namespace without the record fails | refused |
| T31-13 | SUBSCRIBE TOPIC … NAMESPACE | ok |
| T31-14 | PUBLISH TOPIC … NAMESPACE: the namespace's topic | ok |
| T31-15 | PUBLISH without NAMESPACE: the org-wide topic | ok |
| T31-16 | UNSUBSCRIBE within another namespace: not ended | ok |
| T31-17 | UNSUBSCRIBE within its namespace | ok |
| T31-18 | SUBSCRIBE RECALL … NAMESPACE: a live query on that namespace | ok |
| T31-19 | UNSUBSCRIBE the live query within its namespace | ok |

### 32_merge_namespace — ALIASES_NAMESPACE (16 tests, server only)

| Test ID | Checks | Expect |
|---|---|---|
| T32-01 | setup: the survivor in namespace p | ok |
| T32-02 | setup: the duplicate in p | ok |
| T32-03 | setup: notes in p | ok |
| T32-04 | setup: the duplicate wrote the notes, in p | ok |
| T32-05 | setup: the same keys in namespace q | ok |
| T32-06 | setup: the duplicate in q | ok |
| T32-07 | setup: notes in q | ok |
| T32-08 | setup: the duplicate wrote the notes, in q | ok |
| T32-09 | p: the duplicate's key becomes the survivor's alias | ok |
| T32-10 | p: FORGET the duplicate | ok |
| T32-11 | p: the link is the survivor's | ok |
| T32-12 | p: the duplicate's record is gone | ok |
| T32-13 | q: untouched, the duplicate still has its link | ok |
| T32-14 | p: a read by the alias finds the survivor | ok |
| T32-15 | q: the same key is q's own record | ok |
| T32-16 | q: the survivor's key has no alias there, so no link | ok |

### 33_branch_fork_merge — BRANCH_FORK_MERGE (65 tests, server only)

| Test ID | Checks | Expect |
|---|---|---|
| T33-01 | setup: a in the base | ok |
| T33-02 | setup: b in the base | ok |
| T33-03 | the time after the setup (for AS OF below) | ok |
| T33-04 | BRANCH FROM … INTO …: the base as it is now | ok |
| T33-05 | the branch reads the base | ok |
| T33-06 | UPDATE in the branch | ok |
| T33-07 | STORE in the branch | ok |
| T33-08 | the branch sees its own UPDATE | ok |
| T33-09 | the branch sees the base plus its own records | ok |
| T33-10 | the base is unaffected by the branch's UPDATE | ok |
| T33-11 | the base doesn't have the branch's record | ok |
| T33-12 | a base write after the branch was made | ok |
| T33-13 | the branch reads the base as it was at the branch point | ok |
| T33-14 | a branch can be branched again | ok |
| T33-15 | STORE in the branch of the branch | ok |
| T33-16 | MERGE a branch of a branch into its parent | ok |
| T33-17 | the parent has the record | ok |
| T33-18 | FORK: a full, independent copy | ok |
| T33-19 | the fork has the base's records as they are now | ok |
| T33-20 | a base write after the fork | ok |
| T33-21 | the fork doesn't see later base changes | ok |
| T33-22 | UPDATE in the fork | ok |
| T33-23 | the base is unaffected by the fork's UPDATE | ok |
| T33-24 | MERGE FROM branch INTO base: no conflicts (each side changed other records) | ok |
| T33-25 | the base has the branch's UPDATE | ok |
| T33-26 | the base has the branch's records (and its child's) | ok |
| T33-27 | the base keeps its own change | ok |
| T33-28 | setup: branch x | ok |
| T33-29 | setup: branch y | ok |
| T33-30 | setup: branch z | ok |
| T33-31 | the base changes a | ok |
| T33-32 | x changes a too | ok |
| T33-33 | y changes a too | ok |
| T33-34 | MERGE with a conflict and no PREFER fails | refused |
| T33-35 | a failed MERGE writes nothing | ok |
| T33-36 | MERGE … PREFER TARGET: the base wins | ok |
| T33-37 | the base kept its value | ok |
| T33-38 | MERGE … PREFER SOURCE: the branch wins | ok |
| T33-39 | the base has the branch's value | ok |
| T33-40 | STORE in branch z | ok |
| T33-41 | a new base record | ok |
| T33-42 | MERGE FROM base INTO branch: the base's newer changes | ok |
| T33-43 | the branch has the base's new record | ok |
| T33-44 | the branch has the base's newer value | ok |
| T33-45 | the branch keeps its own record | ok |
| T33-46 | a fork can still be merged (the base wins its conflict) | ok |
| T33-47 | the base kept its value | ok |
| T33-48 | BRANCH … AS OF a time | ok |
| T33-49 | the branch reads the base as it was then | ok |
| T33-50 | FORK … AS OF a time | ok |
| T33-51 | the fork has the records as they were then | ok |
| T33-52 | setup: an unrelated namespace | ok |
| T33-53 | two namespaces with no common base can't be merged | refused |
| T33-54 | dropping a namespace that has branches is refused | refused |
| T33-55 | the base is still there | ok |
| T33-56 | drop branch t33-try2 | ok |
| T33-57 | drop branch t33-try | ok |
| T33-58 | drop branch t33-x | ok |
| T33-59 | drop branch t33-y | ok |
| T33-60 | drop branch t33-z | ok |
| T33-61 | drop branch t33-past | ok |
| T33-62 | a dropped branch has no records | ok |
| T33-63 | with its branches dropped, the base can be dropped | ok |
| T33-64 | the base is gone | ok |
| T33-65 | a fork doesn't depend on its base | ok |

