import assert from "node:assert/strict";
import { mkdtemp, writeFile, mkdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import path from "node:path";
import { test } from "node:test";

import {
  buildLumiCommand,
  bundledBinaryPath,
  readJsonFile,
  readTextArtifact,
  resolveOutputFile,
} from "../src/core.js";

test("buildLumiCommand prefers repo-local cargo when lumi-tester/Cargo.toml exists", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "lumi-mcp-"));
  await mkdir(path.join(root, "lumi-tester"));
  await writeFile(path.join(root, "lumi-tester", "Cargo.toml"), "[package]\nname='x'\n");

  const built = buildLumiCommand({
    workspace: root,
    command: "validate",
    args: ["flow.yaml", "--json"],
  });

  assert.deepEqual(built.cmd, "cargo");
  assert.deepEqual(built.args, ["run", "--", "validate", "flow.yaml", "--json"]);
  assert.equal(built.cwd, path.join(root, "lumi-tester"));
  assert.equal(built.kind, "repo");
});

test("buildLumiCommand prefers LUMI_TESTER_BIN over repo-local cargo", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "lumi-mcp-"));
  await mkdir(path.join(root, "lumi-tester"));
  await writeFile(path.join(root, "lumi-tester", "Cargo.toml"), "[package]\nname='x'\n");

  const built = buildLumiCommand({
    workspace: root,
    command: "schema",
    args: ["--json"],
    env: { LUMI_TESTER_BIN: "/opt/lumi/lumi-tester" },
  });

  assert.equal(built.kind, "env");
  assert.equal(built.cmd, "/opt/lumi/lumi-tester");
  assert.deepEqual(built.args, ["schema", "--json"]);
});

test("buildLumiCommand prefers bundled binary over repo-local cargo", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "lumi-mcp-"));
  const packageRoot = await mkdtemp(path.join(tmpdir(), "lumi-pkg-"));
  const bundled = bundledBinaryPath({ packageRoot });
  await mkdir(path.dirname(bundled), { recursive: true });
  await writeFile(bundled, "binary");
  await mkdir(path.join(root, "lumi-tester"));
  await writeFile(path.join(root, "lumi-tester", "Cargo.toml"), "[package]\nname='x'\n");

  const built = buildLumiCommand({
    workspace: root,
    command: "schema",
    packageRoot,
  });

  assert.equal(built.kind, "bundled");
  assert.equal(built.cmd, bundled);
  assert.deepEqual(built.args, ["schema"]);
});

test("resolveOutputFile keeps artifact reads inside output directory", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "lumi-mcp-"));
  const output = path.join(root, "output");
  await mkdir(output);

  assert.equal(resolveOutputFile(output, "run.json"), path.join(output, "run.json"));
  assert.throws(() => resolveOutputFile(output, "../secret.txt"), /outside outputDir/);
});

test("readJsonFile and readTextArtifact return bounded content", async () => {
  const root = await mkdtemp(path.join(tmpdir(), "lumi-mcp-"));
  const jsonPath = path.join(root, "run.json");
  const textPath = path.join(root, "events.jsonl");
  await writeFile(jsonPath, JSON.stringify({ ok: true }));
  await writeFile(textPath, "line1\nline2\nline3\n");

  assert.deepEqual(await readJsonFile(jsonPath), { ok: true });
  assert.equal(await readTextArtifact(textPath, 11), "line1\nline2");
});
