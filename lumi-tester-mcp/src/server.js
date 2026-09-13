#!/usr/bin/env node

import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";
import path from "node:path";

import {
  buildLumiCommand,
  readJsonFile,
  readTextArtifact,
  resolveOutputFile,
  runLumiJson,
  runProcess,
} from "./core.js";

const server = new McpServer({
  name: "lumi-tester-mcp",
  version: "0.1.0",
});

const workspaceSchema = {
  workspace: z.string().optional().describe("Workspace/repo directory. Defaults to process cwd."),
};

function jsonText(value) {
  return {
    content: [{ type: "text", text: JSON.stringify(value, null, 2) }],
  };
}

function textResult(text) {
  return { content: [{ type: "text", text }] };
}

function compactProcessResult(result) {
  return {
    code: result.code,
    timedOut: result.timedOut,
    stdout: trim(result.stdout, 40000),
    stderr: trim(result.stderr, 40000),
    executed: result.executed,
    json: result.json,
  };
}

function trim(text, max) {
  if (!text || text.length <= max) return text || "";
  return `${text.slice(0, max)}\n...<truncated ${text.length - max} chars>`;
}

server.registerTool(
  "validate_yaml",
  {
    title: "Validate Lumi YAML",
    description: "Parse and validate Lumi Tester YAML without launching a device/browser.",
    inputSchema: {
      ...workspaceSchema,
      path: z.string().describe("YAML file or directory to validate."),
    },
  },
  async ({ workspace, path: flowPath }) => {
    const result = await runLumiJson({
      workspace,
      command: "validate",
      args: [flowPath, "--json"],
    });
    return jsonText(compactProcessResult(result));
  },
);

server.registerTool(
  "list_tests",
  {
    title: "List Lumi Tests",
    description: "List discovered Lumi YAML files and command indexes without running tests.",
    inputSchema: {
      ...workspaceSchema,
      path: z.string().describe("YAML file or directory to list."),
    },
  },
  async ({ workspace, path: flowPath }) => {
    const result = await runLumiJson({
      workspace,
      command: "list",
      args: [flowPath, "--json"],
    });
    return jsonText(compactProcessResult(result));
  },
);

server.registerTool(
  "doctor",
  {
    title: "Lumi Doctor",
    description: "Check local Lumi Tester dependencies for one platform.",
    inputSchema: {
      ...workspaceSchema,
      platform: z
        .enum(["android", "android_auto", "ios", "web", "macos", "windows", "all"])
        .default("android"),
    },
  },
  async ({ workspace, platform }) => {
    const result = await runLumiJson({
      workspace,
      command: "doctor",
      args: ["--platform", platform, "--json"],
    });
    return jsonText(compactProcessResult(result));
  },
);

server.registerTool(
  "schema",
  {
    title: "Lumi YAML Schema",
    description: "Return the bundled Lumi YAML JSON Schema.",
    inputSchema: {
      ...workspaceSchema,
    },
  },
  async ({ workspace }) => {
    const result = await runLumiJson({
      workspace,
      command: "schema",
      args: ["--json"],
    });
    return jsonText(compactProcessResult(result));
  },
);

server.registerTool(
  "run_test",
  {
    title: "Run Lumi Test",
    description:
      "Run a Lumi YAML file with report/snapshot/events enabled by default and return process output.",
    inputSchema: {
      ...workspaceSchema,
      path: z.string().describe("YAML test file or directory."),
      platform: z
        .enum(["android", "android_auto", "ios", "web", "macos", "windows"])
        .default("android"),
      output: z.string().default("./output"),
      device: z.string().optional(),
      commandIndex: z.number().int().nonnegative().optional(),
      commandName: z.string().optional(),
      tags: z.array(z.string()).optional(),
      timeoutMs: z.number().int().positive().default(600000),
      report: z.boolean().default(true),
      snapshot: z.boolean().default(true),
      eventsJsonl: z.boolean().default(true),
      continueOnFailure: z.boolean().default(false),
      record: z.boolean().default(false),
    },
  },
  async (args) => {
    const cliArgs = [
      args.path,
      "--platform",
      args.platform,
      "--output",
      args.output,
    ];
    if (args.device) cliArgs.push("--device", args.device);
    if (args.report) cliArgs.push("--report");
    if (args.snapshot) cliArgs.push("--snapshot");
    if (args.eventsJsonl) cliArgs.push("--events-jsonl");
    if (args.continueOnFailure) cliArgs.push("--continue-on-failure");
    if (args.record) cliArgs.push("--record");
    if (args.commandIndex !== undefined) cliArgs.push("--command-index", String(args.commandIndex));
    if (args.commandName) cliArgs.push("--command-name", args.commandName);
    if (args.tags?.length) cliArgs.push("--tags", args.tags.join(","));

    const built = buildLumiCommand({
      workspace: args.workspace,
      command: "run",
      args: cliArgs,
    });
    const result = await runProcess({ ...built, timeoutMs: args.timeoutMs });
    const manifestPath = path.resolve(built.cwd, args.output, "run.json");
    let manifest = null;
    try {
      manifest = await readJsonFile(manifestPath);
    } catch {
      // run may fail before executor finalization; process output remains useful.
    }
    return jsonText({
      code: result.code,
      timedOut: result.timedOut,
      stdout: trim(result.stdout, 40000),
      stderr: trim(result.stderr, 40000),
      executed: built,
      outputDir: path.resolve(built.cwd, args.output),
      manifest,
    });
  },
);

server.registerTool(
  "run_command",
  {
    title: "Run Lumi Command",
    description:
      "Run one or more Lumi Tester commands directly against a device/app session - no YAML file needed. " +
      "Use this for a quick one-off action (tap, type, screenshot, pinch, mockLocation, etc) instead of " +
      "writing and then run_test-ing a throwaway YAML file. Each entry in `commands` uses the exact same " +
      "YAML-sugar syntax as a line in a YAML test file's command list, e.g. 'tapOn: \"Login\"' or " +
      "'pinch: {direction: open, percent: 60}'. All commands in one call share a single device session " +
      "(one connection for all of them, not one per command) - pass several to chain a short sequence.",
    inputSchema: {
      ...workspaceSchema,
      commands: z
        .array(z.string())
        .min(1)
        .describe("One or more commands, same syntax as a YAML test file's command list, run in order."),
      platform: z
        .enum(["android", "ios", "macos", "windows"])
        .default("android")
        .describe("web and android_auto are not supported by this command path - use run_test for those."),
      device: z.string().optional().describe("Device serial (Android) or UDID (iOS)."),
      timeoutMs: z.number().int().positive().default(120000),
    },
  },
  async ({ workspace, commands, platform, device, timeoutMs }) => {
    const cliArgs = ["--platform", platform, "--json"];
    if (device) cliArgs.push("--device", device);
    for (const command of commands) cliArgs.push("--command", command);

    const built = buildLumiCommand({ workspace, command: "shell", args: cliArgs });
    const result = await runProcess({ ...built, timeoutMs });

    // One JSON object per line (not a single JSON value) - `run_one_shot` (Rust
    // side) prints {"command","success","error"} per executed command, interleaved
    // with the driver's own human-readable operational logs (e.g. "📸 Saved
    // screenshot to: ...") on the same stdout stream. Only lines that actually look
    // like JSON are kept - the log lines are expected noise, not parse failures, so
    // they're dropped silently rather than surfacing as fake `parseError` entries.
    const results = result.stdout
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter((line) => line.startsWith("{"))
      .map((line) => {
        try {
          return JSON.parse(line);
        } catch {
          return { parseError: true, line };
        }
      });

    return jsonText({
      code: result.code,
      timedOut: result.timedOut,
      results,
      stderr: trim(result.stderr, 20000),
      executed: built,
    });
  },
);

server.registerTool(
  "read_report",
  {
    title: "Read Lumi Report",
    description: "Read run.json, test-results.json, or another JSON file from an output directory.",
    inputSchema: {
      outputDir: z.string().describe("Lumi output directory."),
      file: z.string().default("run.json"),
    },
  },
  async ({ outputDir, file }) => {
    const resolved = resolveOutputFile(outputDir, file);
    return jsonText(await readJsonFile(resolved));
  },
);

server.registerTool(
  "read_events",
  {
    title: "Read Lumi Events",
    description: "Read and optionally limit events.jsonl from a Lumi output directory.",
    inputSchema: {
      outputDir: z.string().describe("Lumi output directory."),
      file: z.string().default("events.jsonl"),
      limit: z.number().int().positive().max(1000).default(200),
    },
  },
  async ({ outputDir, file, limit }) => {
    const resolved = resolveOutputFile(outputDir, file);
    const text = await readTextArtifact(resolved, 200000);
    const events = text
      .split(/\r?\n/)
      .filter(Boolean)
      .slice(-limit)
      .map((line) => {
        try {
          return JSON.parse(line);
        } catch {
          return { parseError: true, line };
        }
      });
    return jsonText({ events });
  },
);

server.registerTool(
  "read_artifact",
  {
    title: "Read Lumi Artifact",
    description: "Read a bounded text artifact such as failure XML or log from outputDir.",
    inputSchema: {
      outputDir: z.string().describe("Lumi output directory."),
      file: z.string().describe("Artifact file relative to outputDir."),
      maxBytes: z.number().int().positive().max(200000).default(30000),
    },
  },
  async ({ outputDir, file, maxBytes }) => {
    const resolved = resolveOutputFile(outputDir, file);
    return textResult(await readTextArtifact(resolved, maxBytes));
  },
);

server.registerTool(
  "inspector_get",
  {
    title: "Call Lumi Inspector",
    description:
      "Call a running Lumi Inspector REST endpoint, e.g. /api/screenshot, /api/hierarchy, or /api/element-at?x=100&y=200.",
    inputSchema: {
      baseUrl: z.string().default("http://127.0.0.1:9333"),
      endpoint: z.string().describe("Inspector endpoint beginning with /api/."),
    },
  },
  async ({ baseUrl, endpoint }) => {
    if (!endpoint.startsWith("/api/")) {
      throw new Error("endpoint must start with /api/");
    }
    const response = await fetch(new URL(endpoint, baseUrl));
    const text = await response.text();
    try {
      return jsonText({ status: response.status, body: JSON.parse(text) });
    } catch {
      return jsonText({ status: response.status, body: text });
    }
  },
);

server.registerTool(
  "suggest_selectors",
  {
    title: "Suggest Lumi Selectors",
    description:
      "Suggest ranked, cross-platform-correct selectors (Android/iOS/macOS/Windows/Web) for UI elements " +
      "matching a query and/or near a point. Backed by the same SelectorScorer the recorder and Inspector " +
      "use, so index/uniqueness/id-stability are computed from the real element list, not guessed. " +
      "Provide outputDir+file (plus platform) to read a saved hierarchy dump, or omit them to query a " +
      "running Lumi Inspector at baseUrl.",
    inputSchema: {
      ...workspaceSchema,
      platform: z
        .enum(["android", "ios", "macos", "windows", "web"])
        .optional()
        .describe("Required when using outputDir/file. Ignored when querying a running Inspector (it already knows its platform)."),
      outputDir: z.string().optional().describe("Lumi output directory containing a saved hierarchy dump (e.g. a debug artifact)."),
      file: z.string().optional().describe("Hierarchy dump file relative to outputDir. Requires platform."),
      baseUrl: z
        .string()
        .default("http://127.0.0.1:9333")
        .describe("Running Lumi Inspector base URL, used when outputDir/file are omitted."),
      query: z.string().optional().describe("Optional text/id/content-desc/class substring to filter by."),
      point: z.string().optional().describe("Optional 'x,y' point (e.g. from a screenshot click) to prioritize containing/nearby elements."),
      limit: z.number().int().positive().max(50).default(10),
      includeNonClickable: z.boolean().default(false),
    },
  },
  async ({ workspace, platform, outputDir, file, baseUrl, query, point, limit, includeNonClickable }) => {
    if (outputDir && file) {
      if (!platform) {
        throw new Error("platform is required when using outputDir/file");
      }
      const resolved = resolveOutputFile(outputDir, file);
      const args = ["--platform", platform, "--file", resolved, "--limit", String(limit), "--json"];
      if (query) args.push("--query", query);
      if (point) args.push("--point", point);
      if (includeNonClickable) args.push("--include-non-clickable");

      const result = await runLumiJson({ workspace, command: "suggest-selectors", args });
      if (result.json) return jsonText(result.json);
      throw new Error(
        `suggest-selectors failed (exit ${result.code}${result.timedOut ? ", timed out" : ""}): ${result.stderr || result.stdout || "no output"}`,
      );
    }

    const url = new URL("/api/suggest-selectors", baseUrl);
    if (query) url.searchParams.set("query", query);
    if (point) url.searchParams.set("point", point);
    if (limit) url.searchParams.set("limit", String(limit));
    if (includeNonClickable) url.searchParams.set("include_non_clickable", "true");

    const response = await fetch(url);
    const text = await response.text();
    try {
      return jsonText(JSON.parse(text));
    } catch {
      return jsonText({ status: response.status, body: text });
    }
  },
);

const transport = new StdioServerTransport();
await server.connect(transport);
