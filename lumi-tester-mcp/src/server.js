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
  suggestSelectorsFromAndroidXml,
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
      platform: z.enum(["android", "android_auto", "ios", "web", "all"]).default("android"),
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
      platform: z.enum(["android", "android_auto", "ios", "web"]).default("android"),
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
      "Suggest stable Lumi selectors from a UI hierarchy XML artifact. Supports Android UIAutomator XML today.",
    inputSchema: {
      outputDir: z.string().describe("Lumi output directory containing the XML artifact."),
      file: z.string().describe("XML artifact file relative to outputDir."),
      query: z.string().optional().describe("Optional text/id/description/class substring to filter by."),
      point: z.string().optional().describe("Optional absolute point like '540,960' to prioritize containing elements."),
      limit: z.number().int().positive().max(50).default(10),
      includeNonClickable: z.boolean().default(false),
    },
  },
  async ({ outputDir, file, query, point, limit, includeNonClickable }) => {
    const resolved = resolveOutputFile(outputDir, file);
    const xml = await readTextArtifact(resolved, 2_000_000);
    return jsonText(
      suggestSelectorsFromAndroidXml(xml, {
        query,
        point,
        limit,
        includeNonClickable,
      }),
    );
  },
);

const transport = new StdioServerTransport();
await server.connect(transport);
