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
