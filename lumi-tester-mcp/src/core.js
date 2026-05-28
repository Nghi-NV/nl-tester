import { spawn } from "node:child_process";
import { chmod, copyFile, mkdir, readFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const PACKAGE_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

export function findRepoRoot(start = process.cwd()) {
  let current = path.resolve(start);
  while (true) {
    const candidate = path.join(current, "lumi-tester", "Cargo.toml");
    if (existsSync(candidate)) return current;
    const parent = path.dirname(current);
    if (parent === current) return null;
    current = parent;
  }
}

export function bundledBinaryName(platform = process.platform) {
  return platform === "win32" ? "lumi-tester.exe" : "lumi-tester";
}

export function bundledBinaryPath({
  packageRoot = PACKAGE_ROOT,
  platform = process.platform,
  arch = process.arch,
} = {}) {
  return path.join(packageRoot, "binaries", `${platform}-${arch}`, bundledBinaryName(platform));
}

export function resolveLumiBinary({
  workspace = process.cwd(),
  packageRoot = PACKAGE_ROOT,
  env = process.env,
} = {}) {
  if (env.LUMI_TESTER_BIN) {
    return {
      kind: "env",
      cmd: env.LUMI_TESTER_BIN,
      argsPrefix: [],
      cwd: path.resolve(workspace),
    };
  }

  const bundled = bundledBinaryPath({ packageRoot });
  if (existsSync(bundled)) {
    return {
      kind: "bundled",
      cmd: bundled,
      argsPrefix: [],
      cwd: path.resolve(workspace),
    };
  }

  const repoRoot = findRepoRoot(workspace);
  if (repoRoot) {
    return {
      kind: "repo",
      cmd: "cargo",
      argsPrefix: ["run", "--"],
      cwd: path.join(repoRoot, "lumi-tester"),
    };
  }

  return {
    kind: "path",
    cmd: "lumi-tester",
    argsPrefix: [],
    cwd: path.resolve(workspace),
  };
}

export function buildLumiCommand({
  workspace = process.cwd(),
  command,
  args = [],
  packageRoot = PACKAGE_ROOT,
  env = process.env,
}) {
  const resolved = resolveLumiBinary({ workspace, packageRoot, env });
  return {
    kind: resolved.kind,
    cmd: resolved.cmd,
    args: [...resolved.argsPrefix, command, ...args],
    cwd: resolved.cwd,
  };
}

export async function stageLumiBinary({ source, packageRoot = PACKAGE_ROOT } = {}) {
  if (!source) {
    throw new Error("source is required");
  }
  const src = path.resolve(source);
  if (!existsSync(src)) {
    throw new Error(`source binary does not exist: ${src}`);
  }
  const dest = bundledBinaryPath({ packageRoot });
  await mkdir(path.dirname(dest), { recursive: true });
  await copyFile(src, dest);
  if (process.platform !== "win32") {
    await chmod(dest, 0o755);
  }
  return dest;
}

export function runProcess({ cmd, args, cwd, timeoutMs = 120000 }) {
  return new Promise((resolve) => {
    const child = spawn(cmd, args, { cwd, shell: false });
    let stdout = "";
    let stderr = "";
    let timedOut = false;

    const timer = setTimeout(() => {
      timedOut = true;
      child.kill("SIGTERM");
    }, timeoutMs);

    child.stdout.on("data", (chunk) => {
      stdout += chunk.toString();
    });
    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString();
    });
    child.on("error", (error) => {
      clearTimeout(timer);
      resolve({ code: 127, stdout, stderr: `${stderr}${error.message}`, timedOut });
    });
    child.on("close", (code) => {
      clearTimeout(timer);
      resolve({ code: code ?? 1, stdout, stderr, timedOut });
    });
  });
}

export function extractJson(stdout) {
  const trimmed = stdout.trim();
  if (!trimmed) return null;
  const start = Math.min(
    ...["{", "["]
      .map((char) => trimmed.indexOf(char))
      .filter((index) => index >= 0),
  );
  if (!Number.isFinite(start)) return null;
  return JSON.parse(trimmed.slice(start));
}

export async function runLumiJson({ workspace, command, args = [], timeoutMs }) {
  const built = buildLumiCommand({ workspace, command, args });
  const result = await runProcess({ ...built, timeoutMs });
  let json = null;
  try {
    json = extractJson(result.stdout);
  } catch (error) {
    result.stderr = `${result.stderr}\nFailed to parse JSON stdout: ${error.message}`.trim();
  }
  return { ...result, json, executed: built };
}

export async function readJsonFile(file) {
  return JSON.parse(await readFile(file, "utf8"));
}

export async function readTextArtifact(file, maxBytes = 20000) {
  const content = await readFile(file, "utf8");
  return content.length > maxBytes ? content.slice(0, maxBytes) : content;
}

export function resolveOutputFile(outputDir, name) {
  const root = path.resolve(outputDir);
  const file = path.resolve(root, name);
  const rel = path.relative(root, file);
  if (rel.startsWith("..") || path.isAbsolute(rel)) {
    throw new Error(`Refusing to read artifact outside outputDir: ${name}`);
  }
  return file;
}

function decodeXml(value = "") {
  return value
    .replaceAll("&quot;", "\"")
    .replaceAll("&apos;", "'")
    .replaceAll("&lt;", "<")
    .replaceAll("&gt;", ">")
    .replaceAll("&amp;", "&");
}

function parseAttributes(source) {
  const attrs = {};
  const attrRe = /([\w:-]+)="([^"]*)"/g;
  let match;
  while ((match = attrRe.exec(source))) {
    attrs[match[1]] = decodeXml(match[2]);
  }
  return attrs;
}

function parseBounds(bounds) {
  const match = String(bounds || "").match(/\[(\d+),(\d+)\]\[(\d+),(\d+)\]/);
  if (!match) return null;
  const [, left, top, right, bottom] = match.map(Number);
  return {
    left,
    top,
    right,
    bottom,
    width: right - left,
    height: bottom - top,
    centerX: Math.round((left + right) / 2),
    centerY: Math.round((top + bottom) / 2),
  };
}

function containsPoint(bounds, point) {
  if (!bounds || !point) return false;
  return (
    point.x >= bounds.left &&
    point.x <= bounds.right &&
    point.y >= bounds.top &&
    point.y <= bounds.bottom
  );
}

function distanceToPoint(bounds, point) {
  if (!bounds || !point) return Number.POSITIVE_INFINITY;
  return Math.hypot(bounds.centerX - point.x, bounds.centerY - point.y);
}

function normalizeText(value) {
  return String(value || "").trim().toLowerCase();
}

function elementMatchesQuery(element, query) {
  if (!query) return true;
  const needle = normalizeText(query);
  return [
    element.text,
    element.resourceId,
    element.contentDesc,
    element.className,
  ].some((value) => normalizeText(value).includes(needle));
}

function elementFromAttrs(attrs, index) {
  const bounds = parseBounds(attrs.bounds);
  return {
    index,
    text: attrs.text || "",
    resourceId: attrs["resource-id"] || "",
    contentDesc: attrs["content-desc"] || "",
    className: attrs.class || "",
    packageName: attrs.package || "",
    clickable: attrs.clickable === "true",
    enabled: attrs.enabled !== "false",
    selected: attrs.selected === "true",
    bounds,
  };
}

export function parseAndroidUiXml(xml) {
  const elements = [];
  const nodeRe = /<node\b([^>]*)>/g;
  let match;
  while ((match = nodeRe.exec(xml))) {
    elements.push(elementFromAttrs(parseAttributes(match[1]), elements.length));
  }
  return elements;
}

function selectorYaml(selector) {
  const value = selector.value.replaceAll("\\", "\\\\").replaceAll("\"", "\\\"");
  if (selector.type === "id") return `id: "${value}"`;
  if (selector.type === "desc") return `desc: "${value}"`;
  if (selector.type === "text") return `text: "${value}"\nexact: true`;
  if (selector.type === "type") return `type: "${value}"\nindex: ${selector.index || 0}`;
  if (selector.type === "point") return `point: "${value}"`;
  return `${selector.type}: "${value}"`;
}

function candidatesForElement(element) {
  const candidates = [];
  if (element.resourceId) {
    candidates.push({
      type: "id",
      value: element.resourceId,
      score: 100,
      reason: "resource-id is usually the most stable Android selector",
      yaml: selectorYaml({ type: "id", value: element.resourceId }),
    });
  }
  if (element.contentDesc) {
    candidates.push({
      type: "desc",
      value: element.contentDesc,
      score: 90,
      reason: "content-desc/accessibility label is stable when intentionally set",
      yaml: selectorYaml({ type: "desc", value: element.contentDesc }),
    });
  }
  if (element.text) {
    candidates.push({
      type: "text",
      value: element.text,
      score: element.text.length <= 2 ? 55 : 80,
      reason: "visible text is readable but may be localized or duplicated",
      yaml: selectorYaml({ type: "text", value: element.text }),
    });
  }
  if (element.className) {
    candidates.push({
      type: "type",
      value: element.className,
      index: 0,
      score: 40,
      reason: "type is a fallback and should usually be paired with index or anchor",
      yaml: selectorYaml({ type: "type", value: element.className, index: 0 }),
    });
  }
  if (element.bounds) {
    candidates.push({
      type: "point",
      value: `${element.bounds.centerX},${element.bounds.centerY}`,
      score: 10,
      reason: "point is fragile; use only when no semantic selector works",
      yaml: selectorYaml({
        type: "point",
        value: `${element.bounds.centerX},${element.bounds.centerY}`,
      }),
    });
  }
  return candidates.sort((a, b) => b.score - a.score);
}

export function suggestSelectorsFromAndroidXml(xml, {
  query,
  point,
  limit = 10,
  includeNonClickable = false,
} = {}) {
  const parsedPoint =
    typeof point === "string"
      ? (() => {
          const match = point.match(/^\s*(\d+)\s*,\s*(\d+)\s*$/);
          return match ? { x: Number(match[1]), y: Number(match[2]) } : null;
        })()
      : point;

  const elements = parseAndroidUiXml(xml)
    .filter((element) => includeNonClickable || element.clickable || element.text || element.contentDesc)
    .filter((element) => element.enabled)
    .filter((element) => elementMatchesQuery(element, query))
    .map((element) => {
      const candidates = candidatesForElement(element);
      const best = candidates[0] || null;
      const pointBoost = containsPoint(element.bounds, parsedPoint) ? 30 : 0;
      const queryBoost = query && elementMatchesQuery(element, query) ? 10 : 0;
      const clickableBoost = element.clickable ? 5 : 0;
      return {
        ...element,
        distanceToPoint: Number.isFinite(distanceToPoint(element.bounds, parsedPoint))
          ? Math.round(distanceToPoint(element.bounds, parsedPoint))
          : null,
        bestSelector: best,
        selectors: candidates,
        rankScore: (best?.score || 0) + pointBoost + queryBoost + clickableBoost,
      };
    })
    .filter((element) => element.bestSelector);

  elements.sort((a, b) => {
    if (parsedPoint) {
      const aContains = containsPoint(a.bounds, parsedPoint) ? 1 : 0;
      const bContains = containsPoint(b.bounds, parsedPoint) ? 1 : 0;
      if (aContains !== bContains) return bContains - aContains;
      return (a.distanceToPoint ?? 999999) - (b.distanceToPoint ?? 999999);
    }
    return b.rankScore - a.rankScore;
  });

  return {
    count: elements.length,
    suggestions: elements.slice(0, limit).map((element) => ({
      text: element.text,
      resourceId: element.resourceId,
      contentDesc: element.contentDesc,
      className: element.className,
      clickable: element.clickable,
      bounds: element.bounds,
      distanceToPoint: element.distanceToPoint,
      bestSelector: element.bestSelector,
      selectors: element.selectors,
    })),
  };
}
