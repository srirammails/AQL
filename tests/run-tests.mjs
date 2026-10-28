#!/usr/bin/env node
/**
 * AQL Test Suite Runner
 * Runs the YAML conformance tests against the WASM implementation (default), or
 * against any AQL server's `POST /query` (ClawDB, ADB) with `--url <base>`:
 *   node run-tests.mjs --url http://localhost:3000
 * The server should start empty: the runner loads the seed itself.
 */

import { readFileSync, readdirSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { parse as parseYaml } from 'yaml';

const __dirname = dirname(fileURLToPath(import.meta.url));

// Colors for terminal output
const colors = {
  reset: '\x1b[0m',
  green: '\x1b[32m',
  red: '\x1b[31m',
  yellow: '\x1b[33m',
  cyan: '\x1b[36m',
  dim: '\x1b[2m',
};

// --url <base>: run against an HTTP server instead of the WASM build
const urlArg = process.argv.indexOf('--url');
const SERVER_URL = urlArg > 0 ? process.argv[urlArg + 1]?.replace(/\/$/, '') : null;

// HTTP backend with the same interface as the WASM module: execute() returns the
// QueryResult as a JSON string. The server wraps it as {success, data, error}.
function httpBackend(base) {
  return {
    async execute(query) {
      const res = await fetch(`${base}/query`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ query }),
      });
      const body = await res.json();
      const result = body.data && typeof body.data === 'object' && 'success' in body.data
        ? body.data
        : { success: body.success, error: body.error };
      if (!result.success && body.error && !result.error) result.error = body.error;
      return JSON.stringify(result);
    },
    clear() {},
  };
}

// Load WASM module for Node.js
async function loadWasm() {
  const wasmJsPath = join(__dirname, '..', 'pkg', 'clawdb_wasm.js');
  const wasmBinaryPath = join(__dirname, '..', 'pkg', 'clawdb_wasm_bg.wasm');

  // Load the WASM binary
  const wasmBytes = readFileSync(wasmBinaryPath);

  // Import the JS wrapper
  const wasm = await import(wasmJsPath);

  // Initialize with the binary bytes using initSync
  wasm.initSync({ module: wasmBytes });

  return wasm;
}

// Parse seed.aql file into individual statements
function parseSeedFile(content) {
  const lines = content.split('\n');
  const statements = [];
  let current = '';

  for (const line of lines) {
    const trimmed = line.trim();
    // Skip comments and empty lines
    if (trimmed.startsWith('--') || trimmed === '') {
      if (current.trim()) {
        statements.push(current.trim());
        current = '';
      }
      continue;
    }
    current += ' ' + line;
    // Check if statement is complete (ends with ) and next line is empty/comment)
    if (trimmed.endsWith(')') && !trimmed.includes('(')) {
      statements.push(current.trim());
      current = '';
    }
  }
  if (current.trim()) {
    statements.push(current.trim());
  }

  // Better parsing - split on STORE/RECALL/etc keywords
  const cleanStatements = [];
  let buffer = '';
  for (const stmt of statements) {
    const cleaned = stmt.replace(/\s+/g, ' ').trim();
    if (cleaned) {
      cleanStatements.push(cleaned);
    }
  }

  return cleanStatements;
}

// Better seed parser that handles multiline statements
function parseSeedStatements(content) {
  const statements = [];
  const lines = content.split('\n');
  let buffer = '';
  let parenDepth = 0;

  for (const line of lines) {
    const trimmed = line.trim();

    // Skip comments
    if (trimmed.startsWith('--') || trimmed === '') {
      continue;
    }

    buffer += ' ' + trimmed;
    parenDepth += (trimmed.match(/\(/g) || []).length;
    parenDepth -= (trimmed.match(/\)/g) || []).length;

    // Statement is complete when parentheses are balanced
    if (parenDepth === 0 && buffer.trim()) {
      const stmt = buffer.trim().replace(/\s+/g, ' ');
      if (stmt.toUpperCase().startsWith('STORE') ||
          stmt.toUpperCase().startsWith('RECALL') ||
          stmt.toUpperCase().startsWith('LINK') ||
          stmt.toUpperCase().startsWith('UPDATE') ||
          stmt.toUpperCase().startsWith('FORGET')) {
        statements.push(stmt);
      }
      buffer = '';
    }
  }

  return statements;
}

// Run seed data
async function runSeed(wasm) {
  const seedPath = join(__dirname, 'fixtures', 'seed.aql');
  const seedContent = readFileSync(seedPath, 'utf-8');
  const statements = parseSeedStatements(seedContent);

  console.log(`${colors.cyan}Loading seed data (${statements.length} statements)...${colors.reset}`);

  let procId = null;
  for (const stmt of statements) {
    const result = JSON.parse(await wasm.execute(stmt));
    if (!result.success && !stmt.includes('SEMANTIC')) {
      // SEMANTIC stores are read-only in playground, skip those errors
      console.log(`${colors.yellow}Seed warning: ${result.error}${colors.reset}`);
    }
    // Capture PROCEDURAL ID
    if (stmt.includes('PROCEDURAL') && result.success && result.data?.record?.id) {
      procId = result.data.record.id;
    }
  }

  console.log(`${colors.green}Seed data loaded successfully${colors.reset}\n`);
  return procId;
}

// Check if result matches expectation
function checkExpectation(result, expect, testId) {
  const errors = [];

  // Check success/failure
  if (expect.success !== undefined) {
    if (result.success !== expect.success) {
      errors.push(`Expected success=${expect.success}, got ${result.success}`);
      if (!result.success && result.error) {
        errors.push(`Error: ${result.error}`);
      }
    }
  }

  // Check exact count
  if (expect.count !== undefined && result.success) {
    const actualCount = result.data?.count ?? result.data?.records?.length ?? 0;
    if (actualCount !== expect.count) {
      errors.push(`Expected count=${expect.count}, got ${actualCount}`);
    }
  }

  // Check count_gte
  if (expect.count_gte !== undefined && result.success) {
    const actualCount = result.data?.count ?? result.data?.records?.length ?? 0;
    if (actualCount < expect.count_gte) {
      errors.push(`Expected count>=${expect.count_gte}, got ${actualCount}`);
    }
  }

  // Check count_lte
  if (expect.count_lte !== undefined && result.success) {
    const actualCount = result.data?.count ?? result.data?.records?.length ?? 0;
    if (actualCount > expect.count_lte) {
      errors.push(`Expected count<=${expect.count_lte}, got ${actualCount}`);
    }
  }

  // Check error_contains
  if (expect.error_contains && !result.success) {
    const errorMsg = result.error || '';
    if (!errorMsg.toLowerCase().includes(expect.error_contains.toLowerCase())) {
      errors.push(`Error should contain "${expect.error_contains}", got "${errorMsg}"`);
    }
  }

  // Check contains (records that must be present)
  if (expect.contains && result.success && result.data?.records) {
    for (const expected of expect.contains) {
      const found = result.data.records.some(record => {
        // Check in record.data or directly in record
        const data = record.data || record;
        return Object.entries(expected).every(([key, value]) => {
          return data[key] === value || record[key] === value;
        });
      });
      if (!found) {
        errors.push(`Expected record containing ${JSON.stringify(expected)} not found`);
      }
    }
  }

  // Link checks (WITH LINKS results): links on the returned records' `links`.
  const allLinks = () => (result.data?.records ?? []).flatMap(r => (r.data ?? r).links ?? r.links ?? []);
  if (expect.links_count !== undefined && result.success) {
    const n = allLinks().length;
    if (n !== expect.links_count) errors.push(`Expected links_count=${expect.links_count}, got ${n}`);
  }
  if (expect.links_present !== undefined && result.success) {
    const present = allLinks().length > 0;
    if (present !== expect.links_present) errors.push(`Expected links_present=${expect.links_present}, got ${present}`);
  }
  if (expect.links_type_only !== undefined && result.success) {
    const other = allLinks().filter(l => l.link_type !== expect.links_type_only);
    if (other.length) errors.push(`Expected only links of type "${expect.links_type_only}", got ${JSON.stringify(other.map(l => l.link_type))}`);
  }
  if (expect.links_contains && result.success) {
    const links = allLinks();
    for (const want of expect.links_contains) {
      const found = links.some(l => Object.entries(want).every(([k, v]) =>
        typeof v === 'number' && typeof l[k] === 'number' ? Math.abs(l[k] - v) < 1e-6 : l[k] === v));
      if (!found) errors.push(`Expected link containing ${JSON.stringify(want)} not found in ${JSON.stringify(links)}`);
    }
  }

  // Field checks (v0.6 suites): {"data.type": "snapshot", "data.result.data.count": 1}.
  // Paths are dotted, from the QueryResult; `fields_exist` only needs the path present.
  if (expect.fields && result.success) {
    for (let [path, want] of Object.entries(expect.fields)) {
      const got = getPath(result, path);
      if (typeof want === 'string') want = want.replace(/\$\{(\w+)\}/g, (m, n) => (n in captured ? String(captured[n]) : m));
      const same = typeof want === 'number' && typeof got === 'number'
        ? Math.abs(got - want) < 1e-6
        : JSON.stringify(got) === JSON.stringify(want);
      if (!same) errors.push(`Expected ${path}=${JSON.stringify(want)}, got ${JSON.stringify(got)}`);
    }
  }
  if (expect.fields_exist && result.success) {
    for (const path of expect.fields_exist) {
      if (getPath(result, path) === undefined) errors.push(`Expected ${path} to be present`);
    }
  }

  return errors;
}

function getPath(obj, path) {
  return path.split('.').reduce((o, k) => (o == null ? undefined : o[k]), obj);
}

// Values captured from earlier results (`capture: {name: path}`), used in later
// queries as ${name}.
const captured = {};

// Run a single test
async function runTest(wasm, test, procId) {
  let query = test.query.trim();

  // Substitute $PROC_ID if present
  if (procId) {
    query = query.replace(/\$PROC_ID/g, procId);
  }
  query = query.replace(/\$\{(\w+)\}/g, (m, name) => (name in captured ? String(captured[name]) : m));

  try {
    const resultJson = await wasm.execute(query);
    const result = JSON.parse(resultJson);
    const errors = checkExpectation(result, test.expect, test.id);
    for (const [name, path] of Object.entries(test.capture ?? {})) {
      const v = getPath(result, path);
      if (v === undefined) errors.push(`capture ${name}: ${path} not in the result`);
      else captured[name] = v;
    }

    return {
      passed: errors.length === 0,
      errors,
      result,
    };
  } catch (e) {
    return {
      passed: false,
      errors: [`Exception: ${e.message}`],
      result: null,
    };
  }
}

// Run all test suites
async function runAllTests() {
  console.log(`${colors.cyan}╔════════════════════════════════════════════════════════════╗${colors.reset}`);
  console.log(`${colors.cyan}║         AQL Conformance Test Suite Runner (v0.5+)          ║${colors.reset}`);
  console.log(`${colors.cyan}╚════════════════════════════════════════════════════════════╝${colors.reset}\n`);

  // Load WASM, or use the HTTP server given with --url
  let wasm;
  if (SERVER_URL) {
    console.log(`${colors.cyan}Target: ${SERVER_URL}/query${colors.reset}\n`);
    wasm = httpBackend(SERVER_URL);
  } else {
    try {
      wasm = await loadWasm();
    } catch (e) {
      console.error(`${colors.red}Failed to load WASM: ${e.message}${colors.reset}`);
      process.exit(1);
    }
  }

  // Clear any existing data
  wasm.clear();

  // Load seed data
  const procId = await runSeed(wasm);

  // Find all test suite files
  const suitesDir = join(__dirname, 'suites');
  const suiteFiles = readdirSync(suitesDir)
    .filter(f => f.endsWith('.yaml'))
    .sort();

  let totalTests = 0;
  let totalPassed = 0;
  let totalFailed = 0;
  const failedTests = [];

  // Run each suite
  for (const suiteFile of suiteFiles) {
    const suitePath = join(suitesDir, suiteFile);
    const suiteContent = readFileSync(suitePath, 'utf-8');
    const suite = parseYaml(suiteContent);

    // Suites for engine-only statements (v0.6 subscriptions / WATCH / IN SCOPE).
    if (suite.requires === 'server' && !SERVER_URL) {
      console.log(`${colors.dim}━━━ ${suite.suite} (${suiteFile}): skipped, needs --url${colors.reset}\n`);
      continue;
    }
    console.log(`${colors.cyan}━━━ ${suite.suite} (${suiteFile}) ━━━${colors.reset}`);

    let suitePassed = 0;
    let suiteFailed = 0;

    for (const test of suite.tests) {
      totalTests++;
      const result = await runTest(wasm, test, procId);

      if (result.passed) {
        suitePassed++;
        totalPassed++;
        console.log(`  ${colors.green}✓${colors.reset} ${test.id}: ${test.name}`);
      } else {
        suiteFailed++;
        totalFailed++;
        console.log(`  ${colors.red}✗${colors.reset} ${test.id}: ${test.name}`);
        for (const err of result.errors) {
          console.log(`    ${colors.dim}↳ ${err}${colors.reset}`);
        }
        failedTests.push({
          id: test.id,
          name: test.name,
          errors: result.errors,
          query: test.query.trim(),
        });
      }
    }

    const suiteStatus = suiteFailed === 0 ? colors.green : colors.yellow;
    console.log(`${suiteStatus}  Suite: ${suitePassed}/${suitePassed + suiteFailed} passed${colors.reset}\n`);
  }

  // Summary
  console.log(`${colors.cyan}╔════════════════════════════════════════════════════════════╗${colors.reset}`);
  console.log(`${colors.cyan}║                        SUMMARY                             ║${colors.reset}`);
  console.log(`${colors.cyan}╚════════════════════════════════════════════════════════════╝${colors.reset}`);
  console.log(`  Total:  ${totalTests}`);
  console.log(`  ${colors.green}Passed: ${totalPassed}${colors.reset}`);
  console.log(`  ${colors.red}Failed: ${totalFailed}${colors.reset}`);
  console.log(`  Rate:   ${((totalPassed / totalTests) * 100).toFixed(1)}%\n`);

  if (failedTests.length > 0) {
    console.log(`${colors.red}Failed Tests:${colors.reset}`);
    for (const ft of failedTests) {
      console.log(`  ${ft.id}: ${ft.name}`);
    }
  }

  process.exit(totalFailed > 0 ? 1 : 0);
}

runAllTests().catch(e => {
  console.error(`${colors.red}Error: ${e.message}${colors.reset}`);
  process.exit(1);
});
