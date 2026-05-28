#!/usr/bin/env node

import { stageLumiBinary } from "../src/core.js";

const source = process.argv[2] || process.env.LUMI_TESTER_BIN;
if (!source) {
  console.error("Usage: node scripts/stage-binary.js <path-to-lumi-tester-binary>");
  console.error("Or set LUMI_TESTER_BIN=<path-to-lumi-tester-binary>");
  process.exit(2);
}

try {
  const dest = await stageLumiBinary({ source });
  console.log(dest);
} catch (error) {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
}
