import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import * as vscode from 'vscode';

interface PathScalar {
  value: string;
  start: number;
  end: number;
}

function readScalar(line: string, start: number): PathScalar | undefined {
  while (/\s/.test(line[start] ?? '')) start++;
  const quote = line[start];

  if (quote === '"' || quote === "'") {
    let end = start + 1;
    while (end < line.length) {
      if (line[end] === quote) {
        if (quote === "'" && line[end + 1] === "'") {
          end += 2;
          continue;
        }

        let backslashes = 0;
        for (let i = end - 1; i > start && line[i] === '\\'; i--) backslashes++;
        if (quote === "'" || backslashes % 2 === 0) {
          const rawValue = line.slice(start + 1, end);
          let value = rawValue;
          if (quote === "'") {
            value = rawValue.replace(/''/g, "'");
          } else {
            try {
              value = JSON.parse(`"${rawValue}"`);
            } catch {
              // Keep the source spelling if it uses a YAML escape unsupported by JSON.
            }
          }
          return { value, start: start + 1, end };
        }
      }
      end++;
    }
    return undefined;
  }

  let end = start;
  while (end < line.length) {
    const char = line[end];
    const currentValue = line.slice(start, end);
    const hasOpenVariable = currentValue.lastIndexOf('${') > currentValue.lastIndexOf('}');
    if (
      (/[\s,\]]/.test(char) || (char === '}' && !hasOpenVariable)) ||
      (char === '#' && (end === start || /\s/.test(line[end - 1])))
    ) {
      break;
    }
    end++;
  }

  if (end === start) return undefined;
  return { value: line.slice(start, end), start, end };
}

interface EnvironmentValue {
  lineNumber: number;
  name: string;
  section: 'env' | 'vars';
  scalar: PathScalar;
}

function parseEnvironmentValues(document: vscode.TextDocument): EnvironmentValue[] {
  const values: EnvironmentValue[] = [];
  let section: 'env' | 'vars' | undefined;
  let sectionIndent = -1;

  const readMapEntries = (line: string, lineNumber: number, sectionName: 'env' | 'vars', start: number) => {
    let cursor = start;
    while (cursor < line.length) {
      while (/[\s,{]/.test(line[cursor] ?? '')) cursor++;
      if (line[cursor] === '}') break;

      const entry = /^([A-Za-z_][\w.-]*)\s*:\s*/.exec(line.slice(cursor));
      if (!entry) break;
      const scalar = readScalar(line, cursor + entry[0].length);
      if (!scalar) break;
      values.push({ lineNumber, name: entry[1], section: sectionName, scalar });

      cursor = scalar.end;
      const quote = line[scalar.start - 1];
      if ((quote === '"' || quote === "'") && line[cursor] === quote) cursor++;
      if (line[cursor] !== ',') break;
    }
  };

  for (let lineNumber = 0; lineNumber < document.lineCount; lineNumber++) {
    const line = document.lineAt(lineNumber).text;
    const sectionMatch = /^(\s*)(env|vars|var)\s*:\s*(.*)$/i.exec(line);
    if (sectionMatch) {
      section = sectionMatch[2].toLowerCase() === 'env' ? 'env' : 'vars';
      sectionIndent = sectionMatch[1].length;
      const inlineValueStart = line.length - sectionMatch[3].length;
      if (sectionMatch[3].trimStart().startsWith('{')) {
        readMapEntries(line, lineNumber, section, inlineValueStart);
        section = undefined;
      }
      continue;
    }

    if (!section || !line.trim() || line.trimStart().startsWith('#')) continue;
    const indent = /^\s*/.exec(line)?.[0].length ?? 0;
    if (indent <= sectionIndent) {
      section = undefined;
      continue;
    }

    const entry = /^\s*([A-Za-z_][\w.-]*)\s*:\s*/.exec(line);
    if (!entry) continue;
    const scalar = readScalar(line, entry[0].length);
    if (scalar) values.push({ lineNumber, name: entry[1], section, scalar });
  }

  return values;
}

function expandVariables(value: string, variables: Map<string, string>): string | undefined {
  let expanded = value;
  for (let pass = 0; pass < 10; pass++) {
    const next = expanded.replace(/\$\{([^}]+)\}/g, (placeholder, name: string) => {
      const replacement = variables.get(name) ?? process.env[name];
      return replacement === undefined ? placeholder : replacement;
    });
    if (next === expanded) break;
    expanded = next;
  }
  return /\$\{[^}]+\}/.test(expanded) ? undefined : expanded;
}

function resolveFile(
  document: vscode.TextDocument,
  value: string,
  variables: Map<string, string>
): string | undefined {
  const expanded = expandVariables(value.trim(), variables);
  if (!expanded || /^[a-z][a-z\d+.-]*:\/\//i.test(expanded)) {
    return undefined;
  }

  const homeExpanded = expanded === '~'
    ? os.homedir()
    : expanded.startsWith('~/') || expanded.startsWith('~\\')
      ? path.join(os.homedir(), expanded.slice(2))
      : expanded;
  const target = path.isAbsolute(homeExpanded)
    ? path.normalize(homeExpanded)
    : path.resolve(path.dirname(document.uri.fsPath), homeExpanded);

  try {
    return fs.statSync(target).isFile() ? target : undefined;
  } catch {
    return undefined;
  }
}

export class LumiPathLinkProvider implements vscode.DocumentLinkProvider {
  provideDocumentLinks(document: vscode.TextDocument): vscode.DocumentLink[] {
    if (document.uri.scheme !== 'file') return [];

    const links: vscode.DocumentLink[] = [];
    const linkedRanges = new Set<string>();
    const environmentValues = parseEnvironmentValues(document);
    const envVariables = new Map<string, string>();
    const flowVariables = new Map<string, string>();
    for (const item of environmentValues) {
      if (item.name.toLowerCase() === 'file' && item.section === 'env') continue;
      (item.section === 'vars' ? flowVariables : envVariables).set(item.name, item.scalar.value);
    }
    // Flow variables override values in env; both take precedence over process environment.
    const variables = new Map([...envVariables, ...flowVariables]);

    const addLink = (lineNumber: number, scalar: PathScalar | undefined) => {
      if (!scalar) return;
      const targetPath = resolveFile(document, scalar.value, variables);
      if (!targetPath) return;

      const rangeKey = `${lineNumber}:${scalar.start}:${scalar.end}`;
      if (linkedRanges.has(rangeKey)) return;
      linkedRanges.add(rangeKey);

      const link = new vscode.DocumentLink(
        new vscode.Range(lineNumber, scalar.start, lineNumber, scalar.end),
        vscode.Uri.file(targetPath)
      );
      link.tooltip = `Open ${path.basename(targetPath)}`;
      links.push(link);
    };

    for (const item of environmentValues) {
      const isEnvFile = item.section === 'env' && item.name.toLowerCase() === 'file';
      if (isEnvFile || resolveFile(document, item.scalar.value, variables)) {
        addLink(item.lineNumber, item.scalar);
      }
    }

    const pathKeyPattern = /(?:^\s*(?:-\s*)?|\{\s*|,\s*)(?:path|file|pythonPath|savePath|output|image|profile|data)\s*:\s*/g;
    const runFlowPattern = /^\s*(?:-\s*)?runFlow\s*:\s*/i;
    const fileCommandPattern = /^\s*(?:-\s*)?(?:takeScreenshot|screenshot|startRecording|installApp)\s*:\s*/i;

    for (let lineNumber = 0; lineNumber < document.lineCount; lineNumber++) {
      const line = document.lineAt(lineNumber).text;

      pathKeyPattern.lastIndex = 0;
      let match: RegExpExecArray | null;
      while ((match = pathKeyPattern.exec(line)) !== null) {
        addLink(lineNumber, readScalar(line, match.index + match[0].length));
      }

      const runFlowMatch = runFlowPattern.exec(line);
      if (runFlowMatch) {
        const scalar = readScalar(line, runFlowMatch[0].length);
        if (scalar && !scalar.value.startsWith('{')) addLink(lineNumber, scalar);
      }

      const fileCommandMatch = fileCommandPattern.exec(line);
      if (fileCommandMatch) {
        const scalar = readScalar(line, fileCommandMatch[0].length);
        if (scalar && !scalar.value.startsWith('{')) addLink(lineNumber, scalar);
      }
    }

    return links;
  }
}
