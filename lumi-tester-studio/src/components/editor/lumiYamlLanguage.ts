import type { Monaco } from '@monaco-editor/react';
import { parseAllDocuments } from 'yaml';
import { LUMI_COMMANDS, SELECTOR_PARAMS, type CommandParam, type LumiCommand } from '../../../../lumi-tester-vscode/src/schema/commands';

const registered = new WeakSet<object>();
const registeredFileLinks = new WeakSet<object>();
const registeredCodeLenses = new WeakSet<object>();
const registeredFormatters = new WeakSet<object>();
let activeSourcePath: string | null = null;
let activePathOpener: ((sourcePath: string, reference: string) => Promise<void> | void) | null = null;
let linkOpenerRegistered = false;
let activeRunActions: LumiRunActions = {
  runAll: () => undefined,
  runCommand: () => undefined,
  runFromCommand: () => undefined,
};

export interface LumiRunActions {
  runAll: () => void;
  runCommand: (index: number) => void;
  runFromCommand: (index: number) => void;
}

const headerFields = [
  ['platform', 'Target platform: android, ios, web, macos, windows, android_auto'],
  ['appId', 'Android package, iOS bundle ID, or desktop app path'],
  ['url', 'Base URL for Web flows'],
  ['name', 'Flow display name'],
  ['tags', 'Tags used to filter tests'],
  ['env', 'Environment variables; supports a file: path'],
  ['vars', 'Variables; supports a file: path'],
  ['defaultTimeout', 'Default command timeout in milliseconds'],
  ['timeout', 'Alias for defaultTimeout'],
  ['speed', 'Execution speed: turbo, fast, normal, safe'],
  ['browser', 'Web browser: Chrome, Firefox, Webkit'],
  ['device', 'Target device identifier'],
  ['jig', 'Hardware jig connection or profile'],
  ['skip', 'Skip this flow when running a suite'],
  ['manual', 'Mark this flow as manual-only'],
] as const;

const commandsByName = new Map<string, LumiCommand>();
for (const command of LUMI_COMMANDS) {
  commandsByName.set(command.name, command);
  for (const alias of command.aliases ?? []) commandsByName.set(alias, command);
}

const isHeader = (model: any, lineNumber: number) => {
  for (let line = 1; line < lineNumber; line++) {
    if (model.getLineContent(line).trim() === '---') return false;
  }
  return true;
};

const getAncestors = (model: any, currentLine: number) => {
  const ancestors: Array<{ key: string; indent: number }> = [];
  const currentIndent = model.getLineContent(currentLine).match(/^\s*/)?.[0].length ?? 0;
  for (let lineNumber = currentLine - 1; lineNumber >= 1; lineNumber--) {
    const line = model.getLineContent(lineNumber);
    if (line.trim() === '---') break;
    const match = line.match(/^(\s*)(?:-\s*)?([\w-]+):(?:\s|$)/);
    if (!match) continue;
    const indent = match[1].length;
    if (indent >= currentIndent) continue;
    ancestors.push({ key: match[2], indent });
    if (line.trimStart().startsWith('-')) break;
  }
  return ancestors.reverse();
};

const paramSnippet = (param: CommandParam) => param.snippet
  ?? `${param.name}: ${param.type === 'string' ? '"$1"' : param.type === 'boolean' ? '${1|true,false|}' : '$1'}`;

export const registerLumiYamlLanguage = (monaco: Monaco) => {
  if (registered.has(monaco as object)) return;
  registered.add(monaco as object);

  monaco.languages.registerCompletionItemProvider('yaml', {
    triggerCharacters: [':', ' ', '-'],
    provideCompletionItems(model: any, position: any) {
      const line = model.getLineContent(position.lineNumber);
      const word = model.getWordUntilPosition(position);
      const range = {
        startLineNumber: position.lineNumber,
        endLineNumber: position.lineNumber,
        startColumn: word.startColumn,
        endColumn: word.endColumn,
      };

      if (isHeader(model, position.lineNumber)) {
        return {
          suggestions: headerFields.filter(([name]) => !line.includes(`${name}:`)).map(([name, detail]) => ({
            label: name,
            kind: monaco.languages.CompletionItemKind.Property,
            detail,
            documentation: detail,
            insertText: name === 'platform'
              ? 'platform: "${1|android,ios,web,macos,windows,android_auto|}"'
              : name === 'env' || name === 'vars'
                ? name + ':\n  file: ${1:.env}'
                : `${name}: "$1"`,
            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
            range,
          })),
        };
      }

      const indent = line.match(/^\s*/)?.[0].length ?? 0;
      const ancestors = getAncestors(model, position.lineNumber);
      const rootCommand = ancestors[0]?.key ? commandsByName.get(ancestors[0].key) : undefined;
      const nestedKey = ancestors[1]?.key;
      const nestedParams = nestedKey
        ? rootCommand?.params?.find(param => param.name === nestedKey)?.params
        : undefined;
      const insideCommand = !!rootCommand && indent > (ancestors[0]?.indent ?? -1);

      if (insideCommand && rootCommand) {
        const supportsSelectors = (rootCommand.params ?? []).some(param => ['text', 'id', 'regex', 'type'].includes(param.name));
        const params = nestedParams ?? [
          ...(rootCommand.params ?? []),
          ...(supportsSelectors ? SELECTOR_PARAMS : []),
        ];
        const seen = new Set<string>();
        const suggestions = params.filter(param => {
          if (seen.has(param.name)) return false;
          seen.add(param.name);
          return true;
        }).map(param => ({
          label: param.name,
          kind: monaco.languages.CompletionItemKind.Property,
          detail: param.description,
          documentation: param.description,
          insertText: paramSnippet(param),
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          range,
        }));
        return { suggestions };
      }

      const hasDash = line.trimStart().startsWith('-');
      const commandRange = {
        startLineNumber: position.lineNumber,
        endLineNumber: position.lineNumber,
        startColumn: hasDash ? line.indexOf('-') + 1 : word.startColumn,
        endColumn: word.endColumn,
      };
      const suggestions: any[] = [];
      for (const command of LUMI_COMMANDS) {
        for (const name of [command.name, ...(command.aliases ?? [])]) {
          let snippet = command.snippet ?? `${command.name}:`;
          snippet = snippet.replace(new RegExp(`^${command.name}`), name);
          if (command.hasParams && !command.snippet) {
            const firstParam = command.params?.[0];
            snippet += firstParam ? `\n    ${paramSnippet(firstParam)}` : ' "$1"';
          }
          suggestions.push({
            label: name,
            kind: monaco.languages.CompletionItemKind.Function,
            detail: `${command.description} · ${command.category}`,
            documentation: command.description,
            insertText: `- ${snippet}`,
            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
            range: commandRange,
          });
        }
      }
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
  activeRunActions = actions;
  if (registeredCodeLenses.has(monaco as object)) return;
  registeredCodeLenses.add(monaco as object);

  monaco.editor.registerCommand('lumi-studio.runAll', () => activeRunActions.runAll());
  monaco.editor.registerCommand('lumi-studio.runCommand', (_accessor: any, index: number) => activeRunActions.runCommand(index));
  monaco.editor.registerCommand('lumi-studio.runFromCommand', (_accessor: any, index: number) => activeRunActions.runFromCommand(index));

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
      addLens(runAllLine, 'lumi-studio.runAll', '▶ Run All');
      rootCommands.forEach((command, index) => {
        addLens(command.lineIndex, 'lumi-studio.runCommand', `▷ Run [${index}]`, [index]);
        addLens(command.lineIndex, 'lumi-studio.runFromCommand', `▶ Run from [${index}]`, [index]);
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

const pathValuePattern = /\b(?:runFlow|file|path|profile|template|jig|data|route|input|output|snapshot|source|pythonPath|savePath|image)\s*:\s*(?:"([^"]+)"|'([^']+)'|([^\s#,\]}]+))/gi;
const fileCommandPattern = /^\s*(?:-\s*)?(?:takeScreenshot|screenshot|startRecording|installApp)\s*:\s*(?:"([^"]+)"|'([^']+)'|([^\s#,\]}]+))/i;

const isPathValue = (value: string) =>
  !!value && !value.startsWith('${') && !value.includes('://') && !value.startsWith('//')
  && (/[\\/]/.test(value) || /\.[A-Za-z0-9_-]+$/.test(value) || value.startsWith('.') || value.startsWith('~'));

const collectPathReferences = (line: string): PathReference[] => {
  const references: PathReference[] = [];
  const addMatches = (pattern: RegExp) => {
    pattern.lastIndex = 0;
    let match: RegExpExecArray | null;
    while ((match = pattern.exec(line)) !== null) {
      const reference = match[1] ?? match[2] ?? match[3];
      if (reference && isPathValue(reference)) {
        const startIndex = match.index + match[0].lastIndexOf(reference);
        references.push({ reference, startIndex, endIndex: startIndex + reference.length });
      }

      if (!pattern.global) break;
      if (!match[0].length) pattern.lastIndex++;
    }
  };
  addMatches(pathValuePattern);
  addMatches(fileCommandPattern);
  return references;
};

export const lumiYamlPathAt = (line: string, oneBasedColumn: number): string | undefined =>
  collectPathReferences(line).find(reference =>
    oneBasedColumn >= reference.startIndex + 1 && oneBasedColumn <= reference.endIndex + 1,
  )?.reference;

export const registerLumiYamlFileLinks = (
  monaco: Monaco,
  sourcePath: string | null,
  onOpenPath: (reference: string) => Promise<void> | void,
) => {
  activeSourcePath = sourcePath;
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
        const line = model.getLineContent(lineNumber);
        for (const { reference, startIndex, endIndex } of collectPathReferences(line)) {
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
