#!/usr/bin/env node
// Sync the canonical crates.io README to the repository root.
//
// The canonical copy is `crates/ecr17-protocol/README.md` (the crates.io front
// page). Because it uses ABSOLUTE URLs for every badge/image/link, the two files
// are byte-identical — the root mirror is a plain copy, no path rewriting.
//
//   node scripts/sync-readme.mjs          # write the root mirror
//   node scripts/sync-readme.mjs --check  # exit 1 if the root mirror is stale (CI)

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const canonical = resolve(root, "crates/ecr17-protocol/README.md");
const mirror = resolve(root, "README.md");

const source = readFileSync(canonical, "utf8");
const check = process.argv.includes("--check");

if (check) {
  let current = "";
  try {
    current = readFileSync(mirror, "utf8");
  } catch {
    /* missing mirror == stale */
  }
  if (current !== source) {
    console.error(
      "✗ README.md is out of sync with crates/ecr17-protocol/README.md.\n" +
        "  Run: node scripts/sync-readme.mjs",
    );
    process.exit(1);
  }
  console.log("✓ README.md is in sync.");
} else {
  writeFileSync(mirror, source);
  console.log("✓ Wrote README.md from crates/ecr17-protocol/README.md.");
}
