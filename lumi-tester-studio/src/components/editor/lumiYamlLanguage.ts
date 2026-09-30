import type { Monaco } from '@monaco-editor/react';
import { parseAllDocuments } from 'yaml';
import { LUMI_COMMANDS, SELECTOR_PARAMS, type CommandParam, type LumiCommand } from '../../../../lumi-tester-vscode/src/schema/commands';
import { readFile, resolveWorkspaceFileReference } from '../../utils/tauriUtils';

const registered = new WeakSet<object>();
const registeredFileLinks = new WeakSet<object>();
const registeredFormatters = new WeakSet<object>();
let activeSourcePath: string | null = null;
let activePathOpener: ((sourcePath: string, reference: string) => Promise<void> | void) | null = null;
let activeWorkspaceRoot: string | null = null;
let activeWorkspacePaths: string[] = [];
let activeEnvVariablesCache: { key: string; expiresAt: number; names: Promise<string[]> } | null = null;
interface WorkspacePathCandidate {
  label: string;
  normalized: string;
  fileName: string;
  parentCount: number;
  segmentCount: number;
  pathLength: number;
}
let activeWorkspacePathCandidates: WorkspacePathCandidate[] = [];
let linkOpenerRegistered = false;
interface CodeLensRuntime {
  actions: LumiRunActions;
}

const codeLensRuntimeRegistryKey = Symbol.for('lumi-ide.yaml-code-lens-runtimes');
const getCodeLensRuntimeRegistry = () => {
  const globalObject = globalThis as any;
  return (globalObject[codeLensRuntimeRegistryKey] ??= new Map<object, CodeLensRuntime>()) as Map<object, CodeLensRuntime>;
};

export interface LumiRunActions {
  runAll: () => void;
  runCommand: (index: number) => void;
  runFromCommand: (index: number) => void;
}

interface HeaderField {
  name: string;
  detail: string;
  insertText: string;
}

const headerFields: HeaderField[] = [
  { name: 'platform', detail: 'Target platform', insertText: 'platform: "${1|android,ios,web,macos,windows,android_auto|}"' },
  { name: 'appId', detail: 'Android package, iOS bundle ID, or desktop app path', insertText: 'appId: "$1"' },
  { name: 'url', detail: 'Base URL for Web flows', insertText: 'url: "$1"' },
  { name: 'tags', detail: 'Tags used to filter tests', insertText: 'tags:\n  - "$1"' },
  { name: 'env', detail: 'Environment values or a file path relative to this flow', insertText: 'env:\n  file: "${1:.env}"' },
  { name: 'vars', detail: 'Named variables for this flow', insertText: 'vars:\n  ${1:USER_ID}: "$2"' },
  { name: 'data', detail: 'CSV or JSON data file path', insertText: 'data: "$1"' },
  { name: 'defaultTimeout', detail: 'Default command timeout in milliseconds', insertText: 'defaultTimeout: ${1:10000}' },
  { name: 'speed', detail: 'Execution speed: turbo, fast, normal, safe', insertText: 'speed: "${1|turbo,fast,normal,safe|}"' },
  { name: 'browser', detail: 'Web browser: Chrome, Firefox, Webkit', insertText: 'browser: "${1|Chrome,Firefox,Webkit|}"' },
  { name: 'closeWhenFinish', detail: 'Close the browser when the flow finishes', insertText: 'closeWhenFinish: ${1|true,false|}' },
  { name: 'desktopState', detail: 'Desktop state cleanup configuration', insertText: 'desktopState:\n  clear:\n    mode: ${1|autoSafe,manual|}' },
  { name: 'cameras', detail: 'Camera configuration for device-state checks', insertText: 'cameras:\n  ${1:default}:\n    rtsp: "$2"\n    profile: "$3"' },
  { name: 'windowSize', detail: 'Desktop or browser window dimensions', insertText: 'windowSize:\n  width: ${1:1280}\n  height: ${2:800}' },
  { name: 'jig', detail: 'Hardware jig profile path or connection settings', insertText: 'jig: "$1"' },
  { name: 'skip', detail: 'Skip this flow in a bulk run', insertText: 'skip: ${1|true,false|}' },
];

const headerAliases: Record<string, string> = {
  var: 'vars',
  camera: 'cameras',
  hardware: 'jig',
  window: 'windowSize',
  manual: 'skip',
  disabled: 'skip',
  ignore: 'skip',
};

const commandsByName = new Map<string, LumiCommand>();
for (const command of LUMI_COMMANDS) {
  commandsByName.set(command.name, command);
  for (const alias of command.aliases ?? []) commandsByName.set(alias, command);
}

const isHeader = (model: any, lineNumber: number) => {
  for (let line = 1; line <= lineNumber; line++) {
    const content = model.getLineContent(line);
    if (content.trim() === '---' || /^(?:commands|steps):(?:\s|$)/.test(content) || /^-\s*[\w-]+/.test(content)) {
      return false;
    }
  }
  return true;
};

const getDeclaredHeaderFields = (model: any) => {
  const fields = new Set<string>();
  for (let lineNumber = 1; lineNumber <= model.getLineCount(); lineNumber++) {
    const line = model.getLineContent(lineNumber);
    if (line.trim() === '---') break;
    if (!line.trim() || line.trimStart().startsWith('#') || /^\s/.test(line)) continue;
    const key = line.match(/^([\w-]+)\s*:/)?.[1];
    if (key) fields.add(headerAliases[key] ?? key);
  }
  return fields;
};

const getAncestors = (model: any, currentLine: number) => {
  const ancestors: Array<{ key: string; indent: number; lineNumber: number; listItem: boolean }> = [];
  const currentIndent = model.getLineContent(currentLine).match(/^\s*/)?.[0].length ?? 0;
  for (let lineNumber = 1; lineNumber < currentLine; lineNumber++) {
    const line = model.getLineContent(lineNumber);
    if (line.trim() === '---') {
      ancestors.length = 0;
      continue;
    }
    const match = line.match(/^(\s*)(-\s*)?([\w-]+):(?:\s*(.*))?$/);
    if (!match) continue;
    const indent = match[1].length;
    while (ancestors.length && ancestors[ancestors.length - 1].indent >= indent) ancestors.pop();
    const value = match[4]?.trim() ?? '';
    if (!value || value.startsWith('#')) {
      ancestors.push({ key: match[3], indent, lineNumber, listItem: !!match[2] });
    }
  }
  while (ancestors.length && ancestors[ancestors.length - 1].indent >= currentIndent) ancestors.pop();
  return ancestors;
};

const paramSnippet = (param: CommandParam) => param.snippet
  ?? `${param.name}: ${param.type === 'string' ? '"$1"' : param.type === 'boolean' ? '${1|true,false|}' : '$1'}`;

const escapeRegExp = (value: string) => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

const snippetChoicesForField = (snippet: string | undefined, field: string) => {
  if (!snippet) return [];
  const pattern = new RegExp(`(?:^|[\\n,{])\\s*${escapeRegExp(field)}\\s*:\\s*[\"']?\\$\\{\\d+\\|([^{}]+)\\|\\}[\"']?`);
  const choices = pattern.exec(snippet)?.[1];
  return choices ? choices.split(',').map(choice => choice.trim()).filter(Boolean) : [];
};

const descriptionChoices = (description: string) => {
  const list = /:\s*([a-z][\w-]*(?:\s*,\s*[a-z][\w-]*)+)\s*\.?$/i.exec(description)?.[1];
  if (list) return list.split(',').map(value => value.trim());

  const alternatives = /(?:^|:\s*)([a-z][\w-]*)(?:\s+\([^)]*\))?\s+or\s+([a-z][\w-]*)(?:\s+\([^)]*\))?\.?$/i.exec(description);
  return alternatives ? [alternatives[1], alternatives[2]] : [];
};

const commandValueOptions = (command: LumiCommand, field: string, nestedKeys: string[] = []) => {
  const params = getCommandParamsForScope(command, nestedKeys);
  const param = params.find(candidate => candidate.name === field);
  if (!param) return { values: snippetChoicesForField(command.snippet, field), type: 'string' as const };
  if (param.type === 'boolean') return { values: ['true', 'false'], type: 'boolean' as const };
  const selectorSnippet = SELECTOR_PARAMS.find(candidate => candidate.name === field)?.snippet;
  const paramChoices = snippetChoicesForField(param.snippet ?? selectorSnippet, field);
  const commandChoices = snippetChoicesForField(command.snippet, field);
  return {
    values: paramChoices.length
      ? paramChoices
      : commandChoices.length
        ? commandChoices
        : descriptionChoices(param.description),
    type: param.type,
  };
};

const isBooleanChoices = (values: string[]) => values.length > 0
  && values.every(value => value === 'true' || value === 'false');

const indentContinuationLines = (snippet: string, indent: number) => {
  const prefix = ' '.repeat(indent);
  return snippet.split('\n').map((line, index) => index > 0 && line ? `${prefix}${line}` : line).join('\n');
};

const indentSnippetBlock = (snippet: string, indent: number) => {
  const prefix = ' '.repeat(indent);
  return snippet.split('\n').map(line => line ? `${prefix}${line}` : line).join('\n');
};

interface ScalarValueContext {
  key: string;
  startColumn: number;
  endColumn: number;
}

const yamlCommentStartIndex = (line: string) => {
  let quote: '"' | "'" | null = null;
  for (let index = 0; index < line.length; index++) {
    const character = line[index];
    if (quote) {
      if (character === quote && line[index - 1] !== '\\') {
        if (quote === "'" && line[index + 1] === "'") {
          index++;
          continue;
        }
        quote = null;
      }
    } else if (character === '"' || character === "'") {
      quote = character;
    } else if (character === '#' && (index === 0 || /\s/.test(line[index - 1]))) {
      return index;
    }
  }
  return -1;
};

const stripYamlComment = (line: string) => {
  const commentStart = yamlCommentStartIndex(line);
  return commentStart < 0 ? line : line.slice(0, commentStart);
};

const isCursorInYamlComment = (line: string, oneBasedColumn: number) => {
  const commentStart = yamlCommentStartIndex(line);
  return commentStart >= 0 && oneBasedColumn - 1 >= commentStart;
};

const getScalarValueContext = (line: string, oneBasedColumn: number): ScalarValueContext | null => {
  const property = line.match(/^\s*(?:-\s*)?([\w-]+)\s*:(.*)$/);
  if (!property) return null;
  const colonIndex = line.indexOf(':', property.index ?? 0);
  let startIndex = colonIndex + 1;
  while (/\s/.test(line[startIndex] ?? '')) startIndex++;

  let endIndex = startIndex;
  const firstCharacter = line[startIndex];
  if (firstCharacter === '"' || firstCharacter === "'") {
    let closingQuote = -1;
    for (let index = startIndex + 1; index < line.length; index++) {
      if (line[index] === firstCharacter && line[index - 1] !== '\\') {
        closingQuote = index;
        break;
      }
    }
    endIndex = closingQuote >= 0 ? closingQuote + 1 : line.length;
  } else {
    const value = line.slice(startIndex).match(/^[^\s#,}\]]*/)?.[0] ?? '';
    endIndex = startIndex + value.length;
  }

  const cursorIndex = oneBasedColumn - 1;
  return cursorIndex >= startIndex && cursorIndex <= endIndex
    ? { key: property[1], startColumn: startIndex + 1, endColumn: endIndex + 1 }
    : null;
};

interface InlineWhenContext {
  existingKeys: Set<string>;
  isKey: boolean;
  prefix: string;
  startColumn: number;
  endColumn: number;
  hasColon: boolean;
}

const getInlineWhenContext = (line: string, oneBasedColumn: number): InlineWhenContext | null => {
  const cursorIndex = oneBasedColumn - 1;
  const openBraces: number[] = [];
  let quote: '"' | "'" | null = null;

  for (let index = 0; index < cursorIndex && index < line.length; index++) {
    const character = line[index];
    if (quote) {
      if (character === quote && line[index - 1] !== '\\') {
        if (quote === "'" && line[index + 1] === "'") {
          index++;
          continue;
        }
        quote = null;
      }
    } else if (character === '"' || character === "'") {
      quote = character;
    } else if (character === '{') {
      openBraces.push(index);
    } else if (character === '}') {
      openBraces.pop();
    }
  }

  const openIndex = openBraces[openBraces.length - 1];
  if (openIndex === undefined) return null;
  const parent = line.slice(0, openIndex).match(/(?:^|[\s,{])([\w-]+)\s*:\s*$/)?.[1];
  if (parent !== 'when') return null;
  if (/^\s*-\s*when\s*:\s*$/.test(line.slice(0, openIndex))) return null;
  if (quote) {
    return {
      existingKeys: new Set(),
      isKey: false,
      prefix: '',
      startColumn: oneBasedColumn,
      endColumn: oneBasedColumn,
      hasColon: true,
    };
  }

  const segments: Array<{ start: number; end: number }> = [];
  let segmentStart = openIndex + 1;
  let nestedBraces = 0;
  let mapClosed = false;
  quote = null;
  for (let index = segmentStart; index < line.length; index++) {
    const character = line[index];
    if (quote) {
      if (character === quote && line[index - 1] !== '\\') {
        if (quote === "'" && line[index + 1] === "'") {
          index++;
          continue;
        }
        quote = null;
      }
      continue;
    }
    if (character === '"' || character === "'") {
      quote = character;
    } else if (character === '{') {
      nestedBraces++;
    } else if (character === '}') {
      if (nestedBraces === 0) {
        segments.push({ start: segmentStart, end: index });
        mapClosed = true;
        break;
      }
      nestedBraces--;
    } else if (character === ',' && nestedBraces === 0) {
      segments.push({ start: segmentStart, end: index });
      segmentStart = index + 1;
    }
  }
  if (!mapClosed) segments.push({ start: segmentStart, end: line.length });

  const currentIndex = segments.findIndex(segment => cursorIndex >= segment.start && cursorIndex <= segment.end);
  if (currentIndex < 0) return null;
  const current = segments[currentIndex];
  const findColon = (segment: { start: number; end: number }) => {
    let inQuote: '"' | "'" | null = null;
    let depth = 0;
    for (let index = segment.start; index < segment.end; index++) {
      const character = line[index];
      if (inQuote) {
        if (character === inQuote && line[index - 1] !== '\\') inQuote = null;
      } else if (character === '"' || character === "'") {
        inQuote = character;
      } else if (character === '{') {
        depth++;
      } else if (character === '}') {
        depth--;
      } else if (character === ':' && depth === 0) {
        return index;
      }
    }
    return -1;
  };

  const existingKeys = new Set<string>();
  segments.forEach((segment, index) => {
    if (index === currentIndex) return;
    const colon = findColon(segment);
    if (colon < 0) return;
    const key = line.slice(segment.start, colon).trim();
    if (/^[\w-]+$/.test(key)) existingKeys.add(key);
  });

  const colon = findColon(current);
  const keyEnd = colon >= 0 ? colon : current.end;
  const keyStart = current.start + (line.slice(current.start, keyEnd).match(/^\s*/)?.[0].length ?? 0);
  const keyTextEnd = keyStart + (line.slice(keyStart, keyEnd).match(/^[\w-]*/)?.[0].length ?? 0);
  const isKey = colon < 0 || cursorIndex <= colon;
  return {
    existingKeys,
    isKey,
    prefix: line.slice(keyStart, Math.min(cursorIndex, keyTextEnd)).toLowerCase(),
    startColumn: keyStart + 1,
    endColumn: keyTextEnd + 1,
    hasColon: colon >= 0,
  };
};

interface VariableCompletion {
  name: string;
  detail: string;
}

const getInterpolationContext = (line: string, oneBasedColumn: number) => {
  const cursorIndex = oneBasedColumn - 1;
  const openIndex = line.lastIndexOf('${', cursorIndex);
  if (openIndex < 0 || line.slice(openIndex + 2, cursorIndex).includes('}')) return null;

  const closeIndex = line.indexOf('}', openIndex + 2);
  if (closeIndex >= 0 && cursorIndex > closeIndex) return null;
  const expressionEnd = closeIndex < 0 ? line.length : closeIndex;
  const expression = line.slice(openIndex + 2, expressionEnd);
  const cursorExpression = line.slice(openIndex + 2, cursorIndex);
  const variablePart = expression.split(':-', 1)[0];
  if (!/^[\w.]*$/.test(cursorExpression) || !/^[\w.]*$/.test(variablePart)) return null;

  return {
    prefix: cursorExpression.toLowerCase(),
    startColumn: openIndex + 3,
    endColumn: openIndex + 3 + variablePart.length,
  };
};

const getStaticYamlValue = (value: string) => {
  const trimmed = stripYamlComment(value).trim();
  const quoted = trimmed.match(/^(?:"([^"]*)"|'([^']*)')\s*$/);
  const unquoted = trimmed.match(/^([\w.-]+)\s*$/);
  return quoted?.[1] ?? quoted?.[2] ?? unquoted?.[1] ?? null;
};

const getEnvFileReference = (model: any) => {
  const header: string[] = [];
  for (let lineNumber = 1; lineNumber <= model.getLineCount(); lineNumber++) {
    const line = model.getLineContent(lineNumber);
    if (line.trim() === '---') break;
    header.push(line);
  }

  try {
    const document = parseAllDocuments(header.join('\n'))[0];
    const yaml = document?.toJS() as { env?: unknown } | undefined;
    const env = yaml?.env;
    if (!env || typeof env !== 'object' || Array.isArray(env)) return null;
    const reference = (env as Record<string, unknown>).file;
    return typeof reference === 'string' && reference.trim() ? reference : null;
  } catch {
    return null;
  }
};

const parseEnvFileVariableNames = (content: string) => {
  const names = new Set<string>();
  for (const rawLine of content.split(/\r?\n/)) {
    let line = rawLine.trim();
    if (!line || line.startsWith('#')) continue;
    if (line.startsWith('export ')) line = line.slice('export '.length).trimStart();
    const separator = line.indexOf('=');
    if (separator < 0) continue;
    const name = line.slice(0, separator).trim();
    if (/^[A-Za-z0-9_.]+$/.test(name)) names.add(name);
  }
  return [...names];
};

const getEnvFileVariableNames = (sourcePath: string, reference: string) => {
  const key = `${sourcePath}\0${reference}`;
  if (activeEnvVariablesCache?.key === key && activeEnvVariablesCache.expiresAt > Date.now()) {
    return activeEnvVariablesCache.names;
  }

  const names = (async () => {
    try {
      const file = await resolveWorkspaceFileReference(sourcePath, reference);
      return parseEnvFileVariableNames(await readFile(file.path));
    } catch {
      return [];
    }
  })();
  activeEnvVariablesCache = { key, expiresAt: Date.now() + 1000, names };
  return names;
};

const collectInterpolationVariables = (model: any, currentLine: number, usesEnvFile: boolean): VariableCompletion[] => {
  const variables = new Map<string, VariableCompletion>();
  const add = (name: string | null, detail: string) => {
    if (!name || !/^[A-Za-z0-9_.]+$/.test(name) || name === 'file' || variables.has(name)) return;
    variables.set(name, { name, detail });
  };

  for (let lineNumber = 1; lineNumber < currentLine; lineNumber++) {
    const line = model.getLineContent(lineNumber);
    if (line.trim() === '---') {
      break;
    }
    const block = line.match(/^(\s*)(?:env|vars|var)\s*:\s*(.*)$/);
    if (!block) continue;
    if (usesEnvFile && /^\s*env\s*:/.test(line)) continue;
    const parentIndent = block[1].length;
    const inlineMap = block[2].match(/\{([^}]*)\}/)?.[1];
    if (inlineMap) {
      for (const pair of inlineMap.split(',')) add(pair.match(/^\s*([\w.-]+)\s*:/)?.[1] ?? null, 'Flow variable');
    }

    let childIndent: number | null = null;
    for (let childLine = lineNumber + 1; childLine < currentLine; childLine++) {
      const content = model.getLineContent(childLine);
      if (!content.trim() || content.trimStart().startsWith('#')) continue;
      const indent = content.match(/^\s*/)?.[0].length ?? 0;
      if (indent <= parentIndent) break;
      childIndent ??= indent;
      if (indent === childIndent) {
        add(content.match(/^\s*([\w.-]+)\s*:/)?.[1] ?? null, 'Flow variable');
      }
    }
  }

  const variableScopes: Array<{ indent: number; childIndent: number | null }> = [];
  for (let lineNumber = 1; lineNumber < currentLine; lineNumber++) {
    const line = model.getLineContent(lineNumber);
    if (!line.trim() || line.trimStart().startsWith('#')) continue;
    const indent = line.match(/^\s*/)?.[0].length ?? 0;
    while (variableScopes.length && variableScopes[variableScopes.length - 1].indent >= indent) variableScopes.pop();

    const command = line.match(/^(\s*)-\s*([\w-]+)\s*:/);
    if (command) {
      const name = commandsByName.get(command[2])?.name;
      if (name === 'setVar') variableScopes.push({ indent: command[1].length, childIndent: null });
      continue;
    }

    for (const scope of variableScopes) {
      if (indent <= scope.indent) continue;
      scope.childIndent ??= indent;
      if (indent !== scope.childIndent) continue;
      const property = line.match(/^\s*([\w-]+)\s*:(.*)$/);
      if (property?.[1] === 'name') {
        add(getStaticYamlValue(property[2]), 'Set by setVar');
      }
    }
  }

  const ancestors = getAncestors(model, currentLine);
  let activeForEachIndex = -1;
  ancestors.forEach((ancestor, index) => {
    if (commandsByName.get(ancestor.key)?.name === 'forEach') activeForEachIndex = index;
  });
  const forEachBodyKey = activeForEachIndex >= 0 ? ancestors[activeForEachIndex + 1]?.key : undefined;
  const activeForEach = forEachBodyKey === 'commands' || forEachBodyKey === 'do'
    ? ancestors[activeForEachIndex]
    : undefined;
  if (activeForEach) {
    let item: string | null = null;
    let inputIndent: number | null = null;
    let inputPropertyIndent: number | null = null;
    let forEachChildIndent: number | null = null;
    let inInput = false;
    const forEachIndent = activeForEach.indent;
    for (let lineNumber = activeForEach.lineNumber + 1; lineNumber < currentLine; lineNumber++) {
      const line = model.getLineContent(lineNumber);
      if (!line.trim() || line.trimStart().startsWith('#')) continue;
      const indent = line.match(/^\s*/)?.[0].length ?? 0;
      if (indent <= forEachIndent) break;
      forEachChildIndent ??= indent;
      const sibling = line.match(/^\s*([\w-]+)\s*:(.*)$/);
      if (indent === forEachChildIndent && sibling) {
        if (sibling[1] === 'item') item = getStaticYamlValue(sibling[2]);
        inInput = sibling[1] === 'in' || sibling[1] === 'items';
        inputIndent = inInput ? indent : null;
        inputPropertyIndent = null;
        if (inInput) {
          const inlineMap = sibling[2].match(/\{([^}]*)\}/)?.[1];
          for (const pair of inlineMap?.split(',') ?? []) {
            const key = pair.match(/^\s*([\w-]+)\s*:/)?.[1];
            if (item && key) add(`${item}.${key}`, 'forEach item property');
          }
        }
      } else if (inInput && inputIndent !== null && indent > inputIndent) {
        const listProperty = line.match(/^\s*-\s*([\w-]+)\s*:/)?.[1];
        const key = listProperty ?? line.match(/^\s*([\w-]+)\s*:/)?.[1];
        if (listProperty) {
          inputPropertyIndent ??= indent + 2;
          if (item) add(`${item}.${listProperty}`, 'forEach item property');
        } else if (key) {
          inputPropertyIndent ??= indent;
          if (indent === inputPropertyIndent && item) add(`${item}.${key}`, 'forEach item property');
        }
      }
    }
    add(item, 'forEach item');
  }

  add('time', 'Current local time');
  add('date', 'Current date');
  add('timestamp', 'Current UTC timestamp');
  return [...variables.values()];
};

const commandArrayKeys = new Set(['commands', 'steps', 'if', 'then', 'else', 'default']);
const selectorAnchorKeys = new Set(['rightOf', 'leftOf', 'above', 'below']);
const nestedSelectorParams = SELECTOR_PARAMS.filter(param => !['relative', 'scrollable', 'when'].includes(param.name));
const whenConditionParams: CommandParam[] = [
  { name: 'visible', type: 'string', description: 'Run when matching UI text or element is visible', snippet: 'visible: "$1"' },
  { name: 'visibleRegex', type: 'string', description: 'Run when matching UI text is visible', snippet: 'visibleRegex: "$1"' },
  { name: 'notVisible', type: 'string', description: 'Run when matching UI text or element is not visible', snippet: 'notVisible: "$1"' },
  { name: 'notVisibleRegex', type: 'string', description: 'Run when matching UI text is not visible', snippet: 'notVisibleRegex: "$1"' },
];

const inferNestedParamsFromSnippet = (param: CommandParam): CommandParam[] => {
  if (!param.snippet) return [];
  const lines = param.snippet.split('\n');
  const rootPattern = new RegExp(`^(\\s*)${escapeRegExp(param.name)}\\s*:`);
  const rootIndex = lines.findIndex(line => rootPattern.test(line));
  if (rootIndex < 0) return [];
  const rootIndent = lines[rootIndex].match(/^\s*/)?.[0].length ?? 0;
  let childIndent: number | null = null;
  const children: CommandParam[] = [];

  for (const line of lines.slice(rootIndex + 1)) {
    if (!line.trim()) continue;
    const indent = line.match(/^\s*/)?.[0].length ?? 0;
    if (indent <= rootIndent) break;
    childIndent ??= indent;
    if (indent !== childIndent) continue;
    const child = line.match(/^\s*([\w-]+)\s*:(.*)$/);
    if (!child) continue;
    const value = child[2].trim();
    children.push({
      name: child[1],
      type: value ? 'string' : 'object',
      description: `${param.description} · ${child[1]}`,
      snippet: `${child[1]}:${value ? ` ${value}` : ''}`,
    });
  }

  return children;
};

const getCommandParamsForScope = (command: LumiCommand, nestedKeys: string[]): CommandParam[] => {
  const supportsSelectors = (command.params ?? []).some(param => ['text', 'id', 'regex', 'type'].includes(param.name));
  let params = [
    ...(command.params ?? []),
    ...(supportsSelectors ? SELECTOR_PARAMS.filter(param => param.name !== 'when') : []),
  ];
  const whenModifier = SELECTOR_PARAMS.find(param => param.name === 'when');
  if (command.name !== 'when' && whenModifier && !params.some(param => param.name === 'when')) {
    params = [...params, whenModifier];
  }

  for (const key of nestedKeys) {
    const parent = params.find(param => param.name === key && param.params)
      ?? params.find(param => param.name === key);
    if (selectorAnchorKeys.has(key)) {
      params = nestedSelectorParams;
    } else if (key === 'when') {
      params = whenConditionParams;
    } else if (parent) {
      params = parent.params?.length ? parent.params : inferNestedParamsFromSnippet(parent);
    } else {
      return [];
    }
    if (params.length === 0) return [];
  }
  return params;
};

const getSiblingPropertyKeys = (
  model: any,
  parentLine: number,
  parentIndent: number,
  childIndent: number,
  currentLine: number,
) => {
  const keys = new Set<string>();
  for (let lineNumber = parentLine + 1; lineNumber <= model.getLineCount(); lineNumber++) {
    const line = model.getLineContent(lineNumber);
    if (!line.trim() || line.trimStart().startsWith('#')) continue;
    const indent = line.match(/^\s*/)?.[0].length ?? 0;
    if (indent <= parentIndent) break;
    if (lineNumber === currentLine || indent !== childIndent) continue;
    const key = line.match(/^\s*([\w-]+)\s*:/)?.[1];
    if (key) keys.add(key);
  }
  return keys;
};

const pathFields = new Set([
  'file', 'path', 'profile', 'template', 'data', 'route', 'image', 'jig', 'runFlow', 'appId',
  'installApp', 'takeScreenshot', 'screenshot', 'startRecording', 'stopGifCapture',
  'runPython', 'runScript', 'script', 'command', 'output',
  'snapshot', 'source', 'destination', 'savePath',
]);

const barePathFields = new Set([
  'file', 'path', 'profile', 'template', 'data', 'image', 'runFlow', 'takeScreenshot',
  'screenshot', 'startRecording', 'stopGifCapture', 'script',
  'output', 'snapshot', 'source', 'destination', 'savePath',
]);

const transferPathCommands: Record<string, string[]> = {
  source: ['pushFile'],
  destination: ['pullFile'],
};

const isAppIdPathIntent = (value: string) => /[\\/]/.test(value)
  || value.startsWith('.')
  || value.startsWith('~')
  || /\.(?:apk|exe|appx|msi)$/i.test(value);

const isWorkspacePathField = (field: string, commandName?: string) => {
  const requiredCommand = transferPathCommands[field];
  if (!pathFields.has(field)) return false;
  if (field === 'command' && commandName !== 'runScript') return false;
  return !requiredCommand || (!!commandName && requiredCommand.includes(commandName));
};

const commandNameAt = (model: any, lineNumber: number) => {
  const lineCommand = model.getLineContent(lineNumber).match(/^\s*(?:-\s*)?([\w-]+)\s*:/)?.[1];
  const ancestors = getAncestors(model, lineNumber);
  const keys = [lineCommand, ...ancestors.reverse().map(ancestor => ancestor.key)].filter(Boolean) as string[];
  for (const key of keys) {
    const command = commandsByName.get(key);
    if (command) return command.name;
  }
  return undefined;
};

export const lumiYamlCommandNameAt = commandNameAt;

const pathPropertyPattern = /(?:^|[\s,{])([\w-]+)\s*:\s*/g;

interface PathValueContext {
  field: string;
  startColumn: number;
  endColumn: number;
  typed: string;
  shellQuote?: '"' | "'";
  shellAppendQuote?: '"' | "'";
}

interface ShellToken {
  startIndex: number;
  endIndex: number;
  contentStartIndex: number;
  contentEndIndex: number;
  quote?: '"' | "'";
}

const shellTokensInRange = (line: string, startIndex: number, endIndex: number): ShellToken[] => {
  const tokens: ShellToken[] = [];
  let tokenStart = -1;
  let quote: '"' | "'" | null = null;
  let tokenQuote: '"' | "'" | undefined;

  const addToken = (tokenEnd: number) => {
    if (tokenStart < 0) return;
    let contentStartIndex = tokenStart;
    let contentEndIndex = tokenEnd;
    if (line[contentStartIndex] === '"' || line[contentStartIndex] === "'") {
      tokenQuote = line[contentStartIndex] as '"' | "'";
      contentStartIndex++;
      if (line[contentEndIndex - 1] === tokenQuote) contentEndIndex--;
    }
    tokens.push({ startIndex: tokenStart, endIndex: tokenEnd, contentStartIndex, contentEndIndex, quote: tokenQuote });
    tokenStart = -1;
    tokenQuote = undefined;
  };

  for (let index = startIndex; index < endIndex; index++) {
    const character = line[index];
    if (quote) {
      if (character === quote && line[index - 1] !== '\\') quote = null;
      continue;
    }
    if (character === '"' || character === "'") {
      if (line[index - 1] !== '\\') {
        if (tokenStart < 0) tokenStart = index;
        quote = character;
      }
      continue;
    }
    if (/\s/.test(character)) {
      addToken(index);
      continue;
    }
    if (tokenStart < 0) tokenStart = index;
  }
  addToken(endIndex);
  return tokens;
};

interface ShellPathPart {
  reference: string;
  startIndex: number;
  endIndex: number;
  completionStartIndex: number;
  completionEndIndex: number;
  shellQuote?: '"' | "'";
  appendQuote?: '"' | "'";
}

const getShellPathPart = (line: string, token: ShellToken): ShellPathPart => {
  const tokenText = line.slice(token.contentStartIndex, token.contentEndIndex);
  const equalsIndex = tokenText.lastIndexOf('=');
  if (equalsIndex < 0) {
    return {
      reference: tokenText,
      startIndex: token.contentStartIndex,
      endIndex: token.contentEndIndex,
      completionStartIndex: token.startIndex,
      completionEndIndex: token.endIndex,
      shellQuote: token.quote,
    };
  }

  let startIndex = token.contentStartIndex + equalsIndex + 1;
  let endIndex = token.contentEndIndex;
  let appendQuote: '"' | "'" | undefined;
  const quoteOffset = line[startIndex] === '\\' && (line[startIndex + 1] === '"' || line[startIndex + 1] === "'") ? 1 : 0;
  const valueQuote = line[startIndex + quoteOffset] === '"' || line[startIndex + quoteOffset] === "'"
    ? line[startIndex + quoteOffset] as '"' | "'"
    : undefined;
  if (valueQuote) {
    startIndex += quoteOffset + 1;
    if (line[endIndex - 1] === valueQuote && line[endIndex - 2] === '\\') endIndex -= 2;
    else if (line[endIndex - 1] === valueQuote) endIndex--;
    else appendQuote = valueQuote;
  } else if (token.quote && line[token.endIndex - 1] !== token.quote) {
    appendQuote = token.quote;
  }

  return {
    reference: line.slice(startIndex, endIndex),
    startIndex,
    endIndex,
    completionStartIndex: startIndex,
    completionEndIndex: endIndex,
    appendQuote,
  };
};

const isShellPathIntent = (value: string) => /[\\/]/.test(value)
  || value.startsWith('.')
  || value.startsWith('~')
  || /\.[A-Za-z0-9_-]+$/.test(value);

const isWorkspacePathPrefix = (value: string) => {
  const normalized = normalizeTypedWorkspacePath(value);
  return !!normalized && activeWorkspacePathCandidates.some(candidate =>
    candidate.normalized.startsWith(normalized) || candidate.fileName.startsWith(normalized),
  );
};

const getPathValueContext = (line: string, oneBasedColumn: number, commandName?: string): PathValueContext | null => {
  const cursorIndex = oneBasedColumn - 1;
  pathPropertyPattern.lastIndex = 0;
  let match: RegExpExecArray | null;
  while ((match = pathPropertyPattern.exec(line)) !== null) {
    const field = match[1];
    if (!isWorkspacePathField(field, commandName)) continue;

    const valueStartIndex = match.index + match[0].length;
    if (cursorIndex < valueStartIndex) continue;

    const firstValueCharacter = line[valueStartIndex];
    const isShellCommand = (field === 'command' || field === 'runScript') && commandName === 'runScript';
    let endIndex = valueStartIndex;
    let valueContentStartIndex = valueStartIndex;
    let valueContentEndIndex = valueStartIndex;
    let typed = '';
    if (firstValueCharacter === '"' || firstValueCharacter === "'") {
      valueContentStartIndex++;
      let closingQuote = -1;
      for (let index = valueStartIndex + 1; index < line.length; index++) {
        if (line[index] === firstValueCharacter && line[index - 1] !== '\\') {
          closingQuote = index;
          break;
        }
      }
      endIndex = closingQuote >= 0 ? closingQuote + 1 : line.length;
      valueContentEndIndex = closingQuote >= 0 ? closingQuote : endIndex;
    } else if (isShellCommand) {
      endIndex = line.length;
      valueContentEndIndex = endIndex;
    } else {
      const token = line.slice(valueStartIndex).match(/^[^\s#,}\]]*/)?.[0] ?? '';
      endIndex = valueStartIndex + token.length;
      valueContentEndIndex = endIndex;
    }

    if (isShellCommand) {
      const shellContentStart = firstValueCharacter === '"' || firstValueCharacter === "'"
        ? valueContentStartIndex
        : valueStartIndex;
      const shellContentEnd = firstValueCharacter === '"' || firstValueCharacter === "'"
        ? valueContentEndIndex
        : endIndex;
      const token = shellTokensInRange(line, shellContentStart, shellContentEnd)
        .find(candidate => cursorIndex >= candidate.startIndex && cursorIndex <= candidate.endIndex);
      if (!token) continue;
      const pathPart = getShellPathPart(line, token);
      const typed = line.slice(pathPart.startIndex, Math.max(pathPart.startIndex, Math.min(cursorIndex, pathPart.endIndex)));
      const candidatePath = pathPart.reference || typed;
      if (!isShellPathIntent(candidatePath) && !isWorkspacePathPrefix(candidatePath)) continue;
      return {
        field,
        startColumn: pathPart.completionStartIndex + 1,
        endColumn: pathPart.completionEndIndex + 1,
        typed,
        shellQuote: pathPart.shellQuote,
        shellAppendQuote: pathPart.appendQuote,
      };
    }

    if (cursorIndex <= endIndex) {
      typed = line.slice(valueContentStartIndex, Math.max(valueContentStartIndex, Math.min(cursorIndex, valueContentEndIndex)));
      if (field === 'appId' && !isAppIdPathIntent(typed)) continue;
      return { field, startColumn: valueStartIndex + 1, endColumn: endIndex + 1, typed };
    }
  }
  return null;
};

const runScriptBlockHeaderPattern = /^(\s*)(?:-\s*)?(?:command|runScript)\s*:\s*[|>](?:(?:[1-9][+-]?)|(?:[+-][1-9]?))?\s*(?:#.*)?$/;

const isRunScriptBlockContent = (model: any, lineNumber: number) => {
  const line = model.getLineContent(lineNumber);
  if (!line.trim()) return false;
  const currentIndent = line.match(/^\s*/)?.[0].length ?? 0;

  for (let previousLine = lineNumber - 1; previousLine >= 1; previousLine--) {
    const previous = model.getLineContent(previousLine);
    if (!previous.trim()) continue;
    if (previous.trim() === '---') return false;

    const header = previous.match(runScriptBlockHeaderPattern);
    if (header) {
      const headerIndent = header[1].length;
      return currentIndent > headerIndent && commandNameAt(model, previousLine) === 'runScript';
    }

    if (/^\s*-\s*[\w-]+\s*:/.test(previous)
      && commandNameAt(model, previousLine) !== 'runScript') return false;
  }
  return false;
};

const getRunScriptBlockPathContext = (line: string, oneBasedColumn: number) => {
  const prefix = 'command: ';
  const context = getPathValueContext(`${prefix}${line}`, oneBasedColumn + prefix.length, 'runScript');
  return context
    ? { ...context, startColumn: context.startColumn - prefix.length, endColumn: context.endColumn - prefix.length }
    : null;
};

export const setLumiYamlWorkspaceFiles = (root: string | null, paths: string[]) => {
  const normalizedRoot = root?.replace(/\\/g, '/').replace(/\/+$/, '') ?? null;
  const nextRoot = root && normalizedRoot === '' ? '/' : normalizedRoot;
  const changed = activeWorkspaceRoot !== nextRoot || activeWorkspacePaths !== paths;
  activeWorkspaceRoot = nextRoot;
  activeWorkspacePaths = paths;
  if (changed) rebuildWorkspacePathCandidates();
};

const parentDirectoryCount = (path: string) => {
  let count = 0;
  while (path.startsWith('../', count * 3)) count++;
  return count;
};

const pathSegmentCount = (path: string) => {
  let count = path ? 1 : 0;
  for (const character of path) if (character === '/') count++;
  return count;
};

const pathFromSourceDirectory = (targetPath: string) => {
  if (!activeWorkspaceRoot || !activeSourcePath) return null;
  const root = activeWorkspaceRoot;
  const source = activeSourcePath.replace(/\\/g, '/');
  const rootPrefix = root.endsWith('/') ? root : `${root}/`;
  if (!source.startsWith(rootPrefix)) return null;

  const sourceParts = source.slice(rootPrefix.length).split('/').filter(Boolean);
  sourceParts.pop();
  const targetParts = targetPath.replace(/\\/g, '/').split('/').filter(Boolean);
  let sharedParts = 0;
  while (sharedParts < sourceParts.length
    && sharedParts < targetParts.length
    && sourceParts[sharedParts] === targetParts[sharedParts]) {
    sharedParts++;
  }
  return [...Array(sourceParts.length - sharedParts).fill('..'), ...targetParts.slice(sharedParts)].join('/');
};

const rebuildWorkspacePathCandidates = () => {
  activeWorkspacePathCandidates = [];
  if (!activeWorkspaceRoot || !activeSourcePath) return;

  for (const targetPath of activeWorkspacePaths) {
    const relativePath = pathFromSourceDirectory(targetPath);
    if (!relativePath) continue;
    const normalized = relativePath.toLowerCase();
    activeWorkspacePathCandidates.push({
      label: relativePath,
      normalized,
      fileName: normalized.slice(normalized.lastIndexOf('/') + 1),
      parentCount: parentDirectoryCount(relativePath),
      segmentCount: pathSegmentCount(relativePath),
      pathLength: relativePath.length,
    });
  }
};

const normalizeTypedWorkspacePath = (value: string) => {
  let normalized = value.replace(/\\/g, '/');
  if (/^(?:\/|[a-z]:\/)/i.test(normalized) && activeWorkspaceRoot) {
    const root = activeWorkspaceRoot.replace(/\\/g, '/');
    const rootPrefix = root.endsWith('/') ? root : `${root}/`;
    if (normalized.toLowerCase().startsWith(rootPrefix.toLowerCase())) {
      normalized = pathFromSourceDirectory(normalized.slice(rootPrefix.length)) ?? normalized;
    }
  }

  const trailingSlash = normalized.endsWith('/');
  const parts: string[] = [];
  for (const part of normalized.split('/')) {
    if (!part || part === '.') continue;
    if (part === '..' && parts.length && parts[parts.length - 1] !== '..') parts.pop();
    else parts.push(part);
  }
  normalized = parts.join('/');
  return (trailingSlash && normalized ? `${normalized}/` : normalized).toLowerCase();
};

const comparePathSuggestions = (
  left: WorkspacePathCandidate & { matchRank: number },
  right: WorkspacePathCandidate & { matchRank: number },
) => left.matchRank - right.matchRank
  || left.parentCount - right.parentCount
  || left.segmentCount - right.segmentCount
  || left.pathLength - right.pathLength
  || left.label.localeCompare(right.label);

const fuzzyCompletionScore = (candidate: string, query: string) => {
  const value = candidate.toLowerCase();
  const needle = query.toLowerCase();
  if (!needle) return 0;
  if (value === needle) return 1000;
  if (value.startsWith(needle)) return 800 - (value.length - needle.length);
  if (value.includes(needle)) return 500 - value.indexOf(needle);
  let cursor = 0;
  let score = 0;
  let streak = 0;
  for (const character of needle) {
    const index = value.indexOf(character, cursor);
    if (index < 0) return -1;
    streak = index === cursor ? streak + 1 : 0;
    score += 10 + streak * 3 + (index === 0 || /[-_\s]/.test(value[index - 1]) ? 6 : 0);
    cursor = index + 1;
  }
  return score;
};

export const registerLumiYamlLanguage = (monaco: Monaco) => {
  if (registered.has(monaco as object)) return;
  registered.add(monaco as object);

  monaco.languages.registerCompletionItemProvider('yaml', {
    triggerCharacters: [':', ' ', '-', '$', '{', '.', ...'abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_'],
    async provideCompletionItems(model: any, position: any) {
      const line = model.getLineContent(position.lineNumber);
      if (isCursorInYamlComment(line, position.column)) return { suggestions: [] };
      const codeLine = stripYamlComment(line);
      const word = model.getWordUntilPosition(position);
      const range = {
        startLineNumber: position.lineNumber,
        endLineNumber: position.lineNumber,
        startColumn: word.startColumn,
        endColumn: word.endColumn,
      };

      let pathContext = getPathValueContext(codeLine, position.column, commandNameAt(model, position.lineNumber));
      if (!pathContext && isRunScriptBlockContent(model, position.lineNumber)) {
        pathContext = getRunScriptBlockPathContext(codeLine, position.column);
      }
      if (pathContext) {
        const typed = normalizeTypedWorkspacePath(pathContext.typed);
        const rankedSuggestions: Array<WorkspacePathCandidate & { matchRank: number }> = [];
        for (const candidate of activeWorkspacePathCandidates) {
          const matchIndex = typed ? candidate.normalized.indexOf(typed) : 0;
          if (typed && matchIndex < 0) continue;
          const matchRank = !typed ? 0
            : candidate.fileName === typed ? 0
              : candidate.fileName.startsWith(typed) ? 1
                : candidate.normalized.startsWith(typed) ? 2
                  : matchIndex === 0 || /[\\/._-]/.test(candidate.normalized[matchIndex - 1]) ? 3
                    : 4;
          const rankedCandidate = { ...candidate, matchRank };

          let low = 0;
          let high = rankedSuggestions.length;
          while (low < high) {
            const middle = (low + high) >>> 1;
            if (comparePathSuggestions(rankedCandidate, rankedSuggestions[middle]) < 0) high = middle;
            else low = middle + 1;
          }

          if (low >= 100) continue;
          rankedSuggestions.splice(low, 0, rankedCandidate);
          if (rankedSuggestions.length > 100) rankedSuggestions.pop();
        }

        return {
          suggestions: rankedSuggestions.map(({ label }, index) => ({
            label,
            filterText: label,
            sortText: String(index).padStart(3, '0'),
            kind: monaco.languages.CompletionItemKind.File,
            detail: 'Workspace file',
            documentation: `Insert a path relative to ${activeSourcePath?.split(/[\\/]/).pop() ?? 'this flow'}`,
            insertText: pathContext.shellQuote
              ? `${pathContext.shellQuote}${label.split(pathContext.shellQuote).join(`\\${pathContext.shellQuote}`)}${pathContext.shellQuote}`
              : pathContext.shellAppendQuote
                ? `${label}${pathContext.shellAppendQuote}`
              : pathContext.field === 'command' || pathContext.field === 'runScript'
                ? label
                : JSON.stringify(label),
            range: {
              startLineNumber: position.lineNumber,
              endLineNumber: position.lineNumber,
              startColumn: pathContext.startColumn,
              endColumn: pathContext.endColumn,
            },
          })),
        };
      }

      const interpolation = getInterpolationContext(codeLine, position.column);
      if (interpolation) {
        const envFileReference = getEnvFileReference(model);
        const variables = collectInterpolationVariables(model, position.lineNumber, !!envFileReference);
        const sourcePath = activeSourcePath;
        if (envFileReference && sourcePath) {
          const envNames = await getEnvFileVariableNames(sourcePath, envFileReference);
          const knownNames = new Set(variables.map(variable => variable.name));
          for (const name of envNames) {
            if (!knownNames.has(name)) variables.push({ name, detail: `Loaded from ${envFileReference}` });
          }
        }
        return {
          suggestions: variables
            .filter(variable => variable.name.toLowerCase().startsWith(interpolation.prefix))
            .map((variable, index) => ({
              label: variable.name,
              filterText: variable.name,
              sortText: String(index).padStart(3, '0'),
              kind: monaco.languages.CompletionItemKind.Variable,
              detail: variable.detail,
              insertText: variable.name,
              range: {
                startLineNumber: position.lineNumber,
                endLineNumber: position.lineNumber,
                startColumn: interpolation.startColumn,
                endColumn: interpolation.endColumn,
              },
            })),
        };
      }

      const inlineWhen = getInlineWhenContext(codeLine, position.column);
      if (inlineWhen) {
        if (!inlineWhen.isKey) return { suggestions: [] };
        return {
          suggestions: whenConditionParams
            .filter(param => !inlineWhen.existingKeys.has(param.name)
              && param.name.toLowerCase().startsWith(inlineWhen.prefix))
            .map(param => ({
              label: param.name,
              filterText: param.name,
              kind: monaco.languages.CompletionItemKind.Property,
              detail: param.description,
              documentation: param.description,
              insertText: inlineWhen.hasColon ? param.name : paramSnippet(param),
              ...(inlineWhen.hasColon ? {} : {
                insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
              }),
              range: {
                startLineNumber: position.lineNumber,
                endLineNumber: position.lineNumber,
                startColumn: inlineWhen.startColumn,
                endColumn: inlineWhen.endColumn,
              },
            })),
        };
      }

      if (isHeader(model, position.lineNumber)) {
        const currentIndent = line.match(/^\s*/)?.[0].length ?? 0;
        const scalarContext = getScalarValueContext(codeLine, position.column);
        if (scalarContext) {
          const options = headerFields
            .map(field => snippetChoicesForField(field.insertText, scalarContext.key))
            .find(values => values.length > 0) ?? [];
          const typed = line.slice(scalarContext.startColumn - 1, position.column - 1).replace(/^['"]|['"]$/g, '').toLowerCase();
          return {
            suggestions: options.filter(value => !typed || value.toLowerCase().includes(typed)).map(value => ({
              label: value,
              kind: monaco.languages.CompletionItemKind.EnumMember,
              detail: `${scalarContext.key} value`,
              insertText: isBooleanChoices(options) ? value : JSON.stringify(value),
              filterText: value,
              sortText: typed && value.toLowerCase().startsWith(typed) ? `0-${value}` : `1-${value}`,
              range: {
                startLineNumber: position.lineNumber,
                endLineNumber: position.lineNumber,
                startColumn: scalarContext.startColumn,
                endColumn: scalarContext.endColumn,
              },
            })),
          };
        }
        if (currentIndent > 0) return { suggestions: [] };
        return {
          suggestions: headerFields.filter(field => !getDeclaredHeaderFields(model).has(field.name)).map(field => ({
            label: field.name,
            kind: monaco.languages.CompletionItemKind.Property,
            detail: field.detail,
            documentation: field.detail,
            insertText: field.insertText,
            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
            range,
          })),
        };
      }

      const ancestors = getAncestors(model, position.lineNumber);
      const currentIndent = line.match(/^\s*/)?.[0].length ?? 0;
      const currentCommandLine = line.match(/^\s*(-\s*)([\w-]+)\s*:(.*)$/);
      const currentProperty = line.match(/^\s*(?:-\s*)?([\w-]+)\s*:(.*)$/);
      const whenAncestor = [...ancestors].reverse().find(ancestor => ancestor.key === 'when' && !ancestor.listItem);
      if (whenAncestor && currentIndent > whenAncestor.indent) {
        const property = line.match(/^(\s*)([\w-]+)\s*:/);
        const colonIndex = property ? line.indexOf(':', property[1].length) : -1;
        const cursorIndex = position.column - 1;
        if (colonIndex >= 0 && cursorIndex > colonIndex) return { suggestions: [] };

        const keyStart = property ? property[1].length : currentIndent;
        const keyEnd = property
          ? line.slice(0, colonIndex).trimEnd().length
          : keyStart + (line.slice(keyStart).match(/^[\w-]*/)?.[0].length ?? 0);
        const existing = getSiblingPropertyKeys(
          model,
          whenAncestor.lineNumber,
          whenAncestor.indent,
          currentIndent,
          position.lineNumber,
        );
        const prefix = line.slice(keyStart, Math.min(cursorIndex, keyEnd)).toLowerCase();
        return {
          suggestions: whenConditionParams
            .filter(param => !existing.has(param.name) && param.name.toLowerCase().startsWith(prefix))
            .map(param => ({
              label: param.name,
              filterText: param.name,
              kind: monaco.languages.CompletionItemKind.Property,
              detail: param.description,
              documentation: param.description,
              insertText: colonIndex >= 0 ? param.name : paramSnippet(param),
              ...(colonIndex >= 0 ? {} : {
                insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
              }),
              range: {
                startLineNumber: position.lineNumber,
                endLineNumber: position.lineNumber,
                startColumn: keyStart + 1,
                endColumn: keyEnd + 1,
              },
            })),
        };
      }
      const commandAncestorIndex = ancestors.reduce((found, ancestor, index) =>
        ancestor.listItem && commandsByName.has(ancestor.key) ? index : found, -1);
      const commandAncestor = commandAncestorIndex >= 0 ? ancestors[commandAncestorIndex] : undefined;
      const currentCommand = currentCommandLine ? commandsByName.get(currentCommandLine[2]) : undefined;
      const commandArrayAncestorIndex = ancestors.reduce((found, ancestor, index) => {
        if (commandArrayKeys.has(ancestor.key)) return index;
        if (ancestor.key === 'cases' && index < ancestors.length - 1) return index;
        return found;
      }, -1);
      const isRootCommandList = !commandAncestor && ancestors.length === 0 && !isHeader(model, position.lineNumber);
      const isCommandListLocation = isRootCommandList
        || commandArrayAncestorIndex > commandAncestorIndex;

      const makeParamSuggestions = (
        command: LumiCommand,
        nestedKeys: string[],
        parentLine: number,
        parentIndent: number,
        childIndent: number,
        insertPrefix = '',
      ) => {
        const params = getCommandParamsForScope(command, nestedKeys);
        const existing = getSiblingPropertyKeys(model, parentLine, parentIndent, childIndent, position.lineNumber);
        const seen = new Set<string>();
        return params.filter(param => {
          if (seen.has(param.name) || existing.has(param.name)) return false;
          seen.add(param.name);
          return true;
        }).map(param => ({
          label: param.name,
          kind: monaco.languages.CompletionItemKind.Property,
          detail: param.description,
          documentation: param.description,
          insertText: `${insertPrefix}${indentContinuationLines(paramSnippet(param), childIndent)}`,
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          range,
        }));
      };

      if (currentCommand && isCommandListLocation) {
        const valueContext = getScalarValueContext(codeLine, position.column);
        if (valueContext && valueContext.key === currentCommandLine?.[2] && !currentCommandLine?.[3].trim()) {
          return {
            suggestions: makeParamSuggestions(
              currentCommand,
              [],
              position.lineNumber,
              currentIndent,
              currentIndent + 4,
              `\n${' '.repeat(currentIndent + 4)}`,
            ),
          };
        }
      }

      const scalarContext = getScalarValueContext(codeLine, position.column);
      if (scalarContext) {
        const currentKey = scalarContext.key;
        const currentCommandIsProperty = currentProperty && currentProperty[1] === currentKey
          && currentCommand && isCommandListLocation;
        if (currentCommandIsProperty) {
          return { suggestions: [] };
        }

        if (commandAncestor && currentProperty?.[1] === currentKey) {
          const command = commandsByName.get(commandAncestor.key)!;
          const nestedKeys = ancestors.slice(commandAncestorIndex + 1).map(ancestor => ancestor.key);
          const valueOptions = commandValueOptions(command, currentKey, nestedKeys);
          if (valueOptions.values.length > 0) {
            const typed = line.slice(scalarContext.startColumn - 1, position.column - 1).replace(/^['"]|['"]$/g, '').toLowerCase();
            return {
              suggestions: valueOptions.values.filter(value => !typed || value.toLowerCase().includes(typed)).map(value => ({
                label: value,
                kind: monaco.languages.CompletionItemKind.EnumMember,
                detail: `${currentKey} value`,
                insertText: valueOptions.type === 'boolean' || isBooleanChoices(valueOptions.values) ? value : JSON.stringify(value),
                filterText: value,
                sortText: typed && value.toLowerCase().startsWith(typed) ? `0-${value}` : `1-${value}`,
                range: {
                  startLineNumber: position.lineNumber,
                  endLineNumber: position.lineNumber,
                  startColumn: scalarContext.startColumn,
                  endColumn: scalarContext.endColumn,
                },
              })),
            };
          }
          const parentParams = getCommandParamsForScope(
            command,
            [...nestedKeys, currentKey],
          );
          if (parentParams.length > 0) {
            const existing = getSiblingPropertyKeys(model, position.lineNumber, currentIndent, currentIndent + 2, position.lineNumber);
            return {
              suggestions: parentParams.filter(param => !existing.has(param.name)).map(param => ({
                label: param.name,
                kind: monaco.languages.CompletionItemKind.Property,
                detail: param.description,
                documentation: param.description,
                insertText: `\n${' '.repeat(currentIndent + 2)}${indentContinuationLines(paramSnippet(param), currentIndent + 2)}`,
                insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
                range,
              })),
            };
          }
        }
        return { suggestions: [] };
      }

      if (currentCommandLine && isCommandListLocation) {
        // The cursor is in the command name, so let the command completion provider handle it.
      }

      if (commandAncestor && !isCommandListLocation) {
        const command = commandsByName.get(commandAncestor.key)!;
        const nestedKeys = ancestors.slice(commandAncestorIndex + 1).map(ancestor => ancestor.key);
        const parent = ancestors[ancestors.length - 1] ?? commandAncestor;
        const params = getCommandParamsForScope(command, nestedKeys);
        const existing = getSiblingPropertyKeys(model, parent.lineNumber, parent.indent, currentIndent, position.lineNumber);
        const seen = new Set<string>();
        const suggestions = params.filter(param => {
          if (seen.has(param.name) || existing.has(param.name)) return false;
          seen.add(param.name);
          return true;
        }).map(param => ({
          label: param.name,
          kind: monaco.languages.CompletionItemKind.Property,
          detail: param.description,
          documentation: param.description,
          insertText: indentContinuationLines(paramSnippet(param), currentIndent),
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          range,
        }));
        return { suggestions };
      }

      const hasDash = line.trimStart().startsWith('-');
      const commandRange = range;
      const typedCommand = word.word.trim().toLowerCase();
      const recommendedCommands = ['launchApp', 'tap', 'inputText', 'see', 'waitUntilVisible', 'runFlow'];
      const candidates = LUMI_COMMANDS.flatMap(command =>
        [...new Set([command.name, ...(command.aliases ?? [])])].map(name => {
          const matchScore = Math.max(
            fuzzyCompletionScore(name, typedCommand),
            fuzzyCompletionScore(command.description, typedCommand) - 160,
            fuzzyCompletionScore(command.category, typedCommand) - 260,
          );
          const defaultRank = recommendedCommands.indexOf(command.name);
          return { command, name, matchScore, defaultRank: defaultRank < 0 ? recommendedCommands.length : defaultRank };
        }),
      ).filter(item => !typedCommand || item.matchScore >= 0)
        .sort((left, right) => typedCommand
          ? right.matchScore - left.matchScore || left.name.localeCompare(right.name)
          : left.defaultRank - right.defaultRank || left.command.category.localeCompare(right.command.category) || left.name.localeCompare(right.name));

      const suggestions = candidates.map(({ command, name, matchScore }, index) => {
        let snippet = command.snippet ?? `${command.name}:`;
        snippet = snippet.replace(new RegExp(`^${command.name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}`), name);
        if (command.hasParams && !command.snippet) {
          const firstParam = command.params?.[0];
          snippet += firstParam ? `\n${indentSnippetBlock(paramSnippet(firstParam), 4)}` : ' "$1"';
        }
        snippet = indentContinuationLines(snippet, currentIndent);
        const prefix = hasDash
          ? (/\s$/.test(line.slice(0, word.startColumn - 1)) ? '' : ' ')
          : '- ';
        const commandPriority = recommendedCommands.indexOf(command.name);
        const sortRank = typedCommand ? Math.max(0, 1000 - matchScore) : (commandPriority < 0 ? 100 : commandPriority);
        return {
          label: name,
          filterText: `${name} ${command.description} ${command.category}`,
          sortText: `${String(sortRank).padStart(4, '0')}-${String(index).padStart(3, '0')}`,
          kind: monaco.languages.CompletionItemKind.Function,
          detail: `${command.description} · ${command.category}`,
          documentation: command.description,
          insertText: `${prefix}${snippet}`,
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          range: commandRange,
        };
      });
      return { suggestions };
    },
  });

  monaco.languages.registerHoverProvider('yaml', {
    provideHover(model: any, position: any) {
      const word = model.getWordAtPosition(position)?.word;
      const command = word ? commandsByName.get(word) : undefined;
      if (!command) return null;
      return {
        contents: [
          { value: `**${command.name}** · ${command.category}` },
          { value: command.description },
          ...(command.params?.length
            ? [{ value: `Parameters: ${command.params.map(param => `\`${param.name}\``).join(', ')}` }]
            : []),
        ],
      };
    },
  });

  if (!registeredFormatters.has(monaco as object)) {
    registeredFormatters.add(monaco as object);
    monaco.languages.registerDocumentFormattingEditProvider('yaml', {
      provideDocumentFormattingEdits(model: any, options: any) {
        const documents = parseAllDocuments(model.getValue(), { keepSourceTokens: true });
        if (documents.length === 0 || documents.some(document => document.errors.length > 0)) return [];
        const formatted = documents
          .map(document => document.toString({ indent: options.tabSize || 2, lineWidth: 0 }))
          .join('');
        return formatted === model.getValue()
          ? []
          : [{ range: model.getFullModelRange(), text: formatted }];
      },
    });
  }
};

export const registerLumiYamlCodeLenses = (monaco: Monaco, actions: LumiRunActions) => {
  const registry = getCodeLensRuntimeRegistry();
  const existingRuntime = registry.get(monaco as object);
  if (existingRuntime) {
    existingRuntime.actions = actions;
    return;
  }
  const runtime: CodeLensRuntime = { actions };
  registry.set(monaco as object, runtime);

  monaco.editor.registerCommand('lumi-ide.runAll', () => runtime.actions.runAll());
  monaco.editor.registerCommand('lumi-ide.runCommand', (_accessor: any, index: number) => runtime.actions.runCommand(index));
  monaco.editor.registerCommand('lumi-ide.runFromCommand', (_accessor: any, index: number) => runtime.actions.runFromCommand(index));

  monaco.languages.registerCodeLensProvider('yaml', {
    provideCodeLenses(model: any) {
      const lines: string[] = model.getLinesContent();
      const separatorIndex = lines.findIndex(line => line.trim() === '---');
      const startIndex = separatorIndex >= 0 ? separatorIndex + 1 : 0;
      const commands: Array<{ lineIndex: number; indent: number }> = [];

      for (let lineIndex = startIndex; lineIndex < lines.length; lineIndex++) {
        const match = /^(\s*)-\s*[\w-]+(?:\s*:|\s|$)/.exec(lines[lineIndex]);
        if (match) commands.push({ lineIndex, indent: match[1].length });
      }

      const rootIndent = commands.length ? Math.min(...commands.map(command => command.indent)) : 0;
      const rootCommands = commands.filter(command => command.indent === rootIndent);
      const lenses: Array<{ range: any; command: { id: string; title: string; arguments?: unknown[] } }> = [];
      const addLens = (lineIndex: number, id: string, title: string, args?: unknown[]) => {
        const line = lineIndex + 1;
        lenses.push({
          range: new monaco.Range(line, 1, line, 1),
          command: { id, title, arguments: args },
        });
      };

      const runAllLine = separatorIndex >= 0 ? separatorIndex : rootCommands[0]?.lineIndex ?? 0;
      addLens(runAllLine, 'lumi-ide.runAll', '▶ Run All');
      rootCommands.forEach((command, index) => {
        addLens(command.lineIndex, 'lumi-ide.runCommand', `▷ Run [${index}]`, [index]);
        addLens(command.lineIndex, 'lumi-ide.runFromCommand', `▶ Run from [${index}]`, [index]);
      });

      return { lenses };
    },
  });
};

interface PathReference {
  reference: string;
  startIndex: number;
  endIndex: number;
}

const pathValuePattern = /(?:^|[\s,{])([\w-]+)\s*:\s*(?:"([^"]+)"|'([^']+)'|([^\s#,\]}]+))/gi;

const isPathValue = (value: string, field?: string) => {
  if (!value || value.startsWith('${') || value.includes('://') || value.startsWith('//')) return false;
  if (field === 'appId') return isAppIdPathIntent(value);
  return /[\\/]/.test(value) || /\.[A-Za-z0-9_-]+$/.test(value) || value.startsWith('.') || value.startsWith('~')
    || (field !== undefined && barePathFields.has(field));
};

const isKnownWorkspacePath = (reference: string) => {
  const normalized = normalizeTypedWorkspacePath(reference);
  return activeWorkspacePathCandidates.some(candidate => candidate.normalized === normalized);
};

const isWorkspaceAppIdReference = (reference: string) => {
  if (!activeWorkspaceRoot || !activeSourcePath || reference.startsWith('~') || /\.app[\\/]*$/i.test(reference)) return false;
  const rootParts = activeWorkspaceRoot.replace(/\\/g, '/').split('/').filter(Boolean);
  const source = activeSourcePath.replace(/\\/g, '/');
  const rootPrefix = `${activeWorkspaceRoot.replace(/\\/g, '/').replace(/\/+$/, '')}/`;
  if (!source.startsWith(rootPrefix)) return false;

  const normalizedReference = reference.replace(/\\/g, '/');
  const absolute = normalizedReference.startsWith('/') || /^[a-z]:\//i.test(normalizedReference);
  const parts = absolute
    ? normalizedReference.split('/').filter(Boolean)
    : [...rootParts, ...source.slice(rootPrefix.length).split('/').filter(Boolean).slice(0, -1)];
  if (!absolute) {
    for (const part of normalizedReference.split('/')) {
      if (!part || part === '.') continue;
      if (part === '..') {
        if (parts.length <= rootParts.length) return false;
        parts.pop();
      } else {
        parts.push(part);
      }
    }
  }
  return parts.length > rootParts.length && rootParts.every((part, index) => parts[index] === part);
};

const collectPathReferences = (line: string, commandName?: string): PathReference[] => {
  const references: PathReference[] = [];
  const codeLine = stripYamlComment(line);
  const addMatches = (pattern: RegExp) => {
    pattern.lastIndex = 0;
    let match: RegExpExecArray | null;
    while ((match = pattern.exec(codeLine)) !== null) {
      const isPathProperty = pattern === pathValuePattern;
      const field = isPathProperty ? match[1] : undefined;
      const reference = isPathProperty ? match[2] ?? match[3] ?? match[4] : match[1] ?? match[2] ?? match[3];
      if (field && ((field === 'command' && commandName === 'runScript')
        || (field === 'runScript' && commandName === 'runScript'))) {
        continue;
      }
      if (reference
        && (!field || isWorkspacePathField(field, commandName))
        && isPathValue(reference, field)
        && (field !== 'appId' || isWorkspaceAppIdReference(reference))) {
        const startIndex = match.index + match[0].lastIndexOf(reference);
        references.push({ reference, startIndex, endIndex: startIndex + reference.length });
      }

      if (!pattern.global) break;
      if (!match[0].length) pattern.lastIndex++;
    }
  };
  addMatches(pathValuePattern);

  if (commandName === 'runScript') {
    pathPropertyPattern.lastIndex = 0;
    let match: RegExpExecArray | null;
    while ((match = pathPropertyPattern.exec(codeLine)) !== null) {
      const field = match[1];
      if (field !== 'command' && field !== 'runScript') continue;
      const valueStart = match.index + match[0].length;
      const yamlQuote = codeLine[valueStart] === '"' || codeLine[valueStart] === "'"
        ? codeLine[valueStart] as '"' | "'"
        : undefined;
      let contentStart = valueStart;
      let contentEnd = codeLine.length;
      if (yamlQuote) {
        contentStart++;
        let closingQuote = -1;
        for (let index = contentStart; index < codeLine.length; index++) {
          if (codeLine[index] === yamlQuote && codeLine[index - 1] !== '\\') {
            closingQuote = index;
            break;
          }
        }
        if (closingQuote >= 0) contentEnd = closingQuote;
      }

      for (const token of shellTokensInRange(codeLine, contentStart, contentEnd)) {
        const pathPart = getShellPathPart(codeLine, token);
        const { reference } = pathPart;
        if (!reference || !isShellPathIntent(reference) || !isKnownWorkspacePath(reference)) continue;
        references.push({
          reference,
          startIndex: pathPart.startIndex,
          endIndex: pathPart.endIndex,
        });
      }
    }
  }
  return references;
};

const collectShellPathReferences = (line: string): PathReference[] => {
  const codeLine = stripYamlComment(line);
  const references: PathReference[] = [];
  for (const token of shellTokensInRange(codeLine, 0, codeLine.length)) {
    const pathPart = getShellPathPart(codeLine, token);
    const { reference } = pathPart;
    if (!reference || !isKnownWorkspacePath(reference)) continue;
    references.push({
      reference,
      startIndex: pathPart.startIndex,
      endIndex: pathPart.endIndex,
    });
  }
  return references;
};

const collectModelPathReferences = (model: any, lineNumber: number): PathReference[] => {
  const line = model.getLineContent(lineNumber);
  if (isRunScriptBlockContent(model, lineNumber)) return collectShellPathReferences(line);
  return collectPathReferences(line, commandNameAt(model, lineNumber));
};

export const lumiYamlPathAt = (line: string, oneBasedColumn: number, commandName?: string): string | undefined =>
  collectPathReferences(line, commandName).find(reference =>
    oneBasedColumn >= reference.startIndex + 1 && oneBasedColumn <= reference.endIndex + 1,
  )?.reference;

export const lumiYamlPathAtModel = (model: any, lineNumber: number, oneBasedColumn: number): string | undefined =>
  collectModelPathReferences(model, lineNumber).find(reference =>
    oneBasedColumn >= reference.startIndex + 1 && oneBasedColumn <= reference.endIndex + 1,
  )?.reference;

export const registerLumiYamlFileLinks = (
  monaco: Monaco,
  sourcePath: string | null,
  onOpenPath: (reference: string) => Promise<void> | void,
) => {
  const sourceChanged = activeSourcePath !== sourcePath;
  activeSourcePath = sourcePath;
  if (sourceChanged) rebuildWorkspacePathCandidates();
  activePathOpener = (source, reference) => {
    if (source === activeSourcePath) return onOpenPath(reference);
  };

  if (!linkOpenerRegistered) {
    linkOpenerRegistered = true;
    monaco.editor.registerLinkOpener({
      open(resource: any) {
        if (resource.scheme !== 'lumi-workspace') return false;
        const params = new URLSearchParams(resource.query);
        const source = params.get('source');
        const reference = params.get('reference');
        if (!source || !reference || !activePathOpener) return true;
        Promise.resolve(activePathOpener(source, reference)).catch(error => {
          window.alert(`Could not open referenced file: ${String(error)}`);
        });
        return true;
      },
    });
  }

  if (registeredFileLinks.has(monaco as object)) return;
  registeredFileLinks.add(monaco as object);
  monaco.languages.registerLinkProvider('yaml', {
    provideLinks(model: any) {
      if (!activeSourcePath) return { links: [] };
      const links: any[] = [];
      for (let lineNumber = 1; lineNumber <= model.getLineCount(); lineNumber++) {
        for (const { reference, startIndex, endIndex } of collectModelPathReferences(model, lineNumber)) {
          links.push({
            range: {
              startLineNumber: lineNumber,
              startColumn: startIndex + 1,
              endLineNumber: lineNumber,
              endColumn: endIndex + 1,
            },
            target: monaco.Uri.from({
              scheme: 'lumi-workspace',
              path: '/file',
              query: `source=${encodeURIComponent(activeSourcePath)}&reference=${encodeURIComponent(reference)}`,
            }),
            tooltip: `Open ${reference}`,
          });
        }
      }
      return { links };
    },
  });
};
