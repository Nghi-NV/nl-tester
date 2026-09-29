import React, { useEffect, useRef } from 'react';
import type { Monaco } from '@monaco-editor/react';
import * as monacoApi from 'monaco-editor/esm/vs/editor/editor.api.js';
import type { editor } from 'monaco-editor';
import { defineCodeverseTheme, createStepDecorations, EXECUTING_CSS } from './monacoUtils';
import { lumiYamlPathAtModel, registerLumiYamlCodeLenses, registerLumiYamlFileLinks, registerLumiYamlLanguage, setLumiYamlWorkspaceFiles } from './lumiYamlLanguage';

interface EditorCoreProps {
  value: string;
  onChange: (value: string) => void;
  formatRequest?: number;
  readOnly?: boolean;
  onRunAll?: () => void;
  onRunCommand?: (index: number) => void;
  onRunFromCommand?: (index: number) => void;
  // Executing line number (0-indexed) for debugging highlight
  executingLine?: number;
  // Whether tests are currently running
  isRunning?: boolean;
  // Step statuses map (step index -> status)
  stepStatuses?: Map<number, 'running' | 'passed' | 'failed' | 'pending' | 'cancelled'>;
  // Step lines map (step index -> line number) from execution state store
  stepLinesMap?: Map<number, number>;
  // Step errors map (step index -> error message)
  stepErrors?: Map<number, string>;
  // Callback when failed icon is clicked
  onFailedStepClick?: (stepIndex: number, error: string, lineNumber: number) => void;
  language?: string;
  sourcePath?: string | null;
  workspaceRoot?: string | null;
  workspacePaths?: string[];
  onOpenPath?: (reference: string) => Promise<void> | void;
  diagnostics?: Array<{ message: string; line?: number; column?: number }>;
  revealPosition?: { fileId: string; lineNumber: number; column: number };
  onRevealComplete?: (fileId: string) => void;
}

export const EditorCore: React.FC<EditorCoreProps> = ({
  value,
  onChange,
  formatRequest = 0,
  readOnly = false,
  onRunAll = () => undefined,
  onRunCommand = () => undefined,
  onRunFromCommand = () => undefined,
  executingLine = -1,
  isRunning = false,
  stepStatuses,
  stepLinesMap,
  stepErrors,
  onFailedStepClick,
  language = 'yaml',
  sourcePath = null,
  workspaceRoot = null,
  workspacePaths = [],
  onOpenPath = () => undefined,
  diagnostics = [],
  revealPosition,
  onRevealComplete,
}) => {
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const isApplyingValueRef = useRef(false);
  const pendingRevealRef = useRef(revealPosition);
  const onRevealCompleteRef = useRef(onRevealComplete);
  const sourcePathRef = useRef(sourcePath);
  const valueRef = useRef(value);
  const onChangeRef = useRef(onChange);
  const onOpenPathRef = useRef(onOpenPath);
  const stepLinesMapRef = useRef(stepLinesMap);
  const stepErrorsRef = useRef(stepErrors);
  const monaco = monacoApi as Monaco;
  const diagnosticsRef = useRef(diagnostics);
  diagnosticsRef.current = diagnostics;
  stepLinesMapRef.current = stepLinesMap;
  stepErrorsRef.current = stepErrors;

  const applyDiagnostics = () => {
    const model = editorRef.current?.getModel();
    if (!model) return;
    monaco.editor.setModelMarkers(model, 'lumi-tester', diagnosticsRef.current.map(diagnostic => {
      const startLineNumber = Math.max(1, Math.min(model.getLineCount(), diagnostic.line ?? 1));
      const maxColumn = model.getLineMaxColumn(startLineNumber);
      const startColumn = Math.max(1, Math.min(maxColumn, diagnostic.column ?? 1));
      return {
        severity: monaco.MarkerSeverity.Error,
        message: diagnostic.message,
        startLineNumber,
        endLineNumber: startLineNumber,
        startColumn,
        endColumn: Math.min(maxColumn, startColumn + 1),
        source: 'Lumi Tester',
      };
    }));
  };

  useEffect(() => {
    valueRef.current = value;
  }, [value]);

  useEffect(() => {
    onChangeRef.current = onChange;
  }, [onChange]);

  useEffect(() => {
    onRevealCompleteRef.current = onRevealComplete;
  }, [onRevealComplete]);

  useEffect(() => {
    sourcePathRef.current = sourcePath;
  }, [sourcePath]);

  useEffect(() => {
    onOpenPathRef.current = onOpenPath;
  }, [onOpenPath]);

  useEffect(() => {
    editorRef.current?.trigger('format-button', 'editor.action.formatDocument', null);
  }, [formatRequest]);

  const applyPendingReveal = () => {
    const currentEditor = editorRef.current;
    const pending = pendingRevealRef.current;
    const model = currentEditor?.getModel();
    const normalizeEol = (text: string) => text.replace(/\r\n|\r/g, '\n');
    if (!currentEditor || !model || !pending || normalizeEol(model.getValue()) !== normalizeEol(valueRef.current)) return;
    const lineNumber = Math.max(1, Math.min(pending.lineNumber, model.getLineCount()));
    const column = Math.max(1, Math.min(pending.column, model.getLineMaxColumn(lineNumber)));
    currentEditor.revealLineInCenter(lineNumber);
    currentEditor.setPosition({ lineNumber, column });
    currentEditor.focus();
    pendingRevealRef.current = undefined;
    onRevealCompleteRef.current?.(pending.fileId);
  };

  useEffect(() => {
    pendingRevealRef.current = revealPosition;
    applyPendingReveal();
  }, [revealPosition, value]);

  useEffect(() => {
    if (monaco && language === 'yaml') {
      registerLumiYamlFileLinks(monaco, sourcePath, onOpenPath);
    }
  }, [monaco, language, sourcePath, onOpenPath]);

  useEffect(() => {
    if (!monaco || language !== 'yaml') return;
    registerLumiYamlCodeLenses(monaco, { runAll: onRunAll, runCommand: onRunCommand, runFromCommand: onRunFromCommand });
  }, [monaco, language, onRunAll, onRunCommand, onRunFromCommand]);

  useEffect(() => {
    setLumiYamlWorkspaceFiles(workspaceRoot, workspacePaths);
  }, [workspaceRoot, workspacePaths]);

  useEffect(() => {
    applyDiagnostics();
  }, [monaco, diagnostics, value]);

  // Initialize Theme and Completion
  useEffect(() => {
    if (monaco) {
      defineCodeverseTheme(monaco);
      monaco.editor.setTheme('codeverse-dark');

      // Register YAML completions
      registerLumiYamlLanguage(monaco);
    }
  }, [monaco]);

  // Update Language dynamically
  useEffect(() => {
    if (monaco && editorRef.current) {
      const model = editorRef.current.getModel();
      if (model) {
        monaco.editor.setModelLanguage(model, language);
        editorRef.current.updateOptions({ wordBasedSuggestions: language === 'yaml' ? 'off' : 'allDocuments' });
      }
    }
  }, [monaco, language]);

  // Handle execution highlights and result decorations.
  useEffect(() => {
    if (!editorRef.current || !monaco) return;

    const model = editorRef.current.getModel();
    if (!model) return;

    // Only highlight executing line when actually running
    const effectiveExecutingLine = isRunning && executingLine >= 0 ? executingLine : -1;
    const decorations = createStepDecorations(effectiveExecutingLine, stepStatuses, stepLinesMap, stepErrors);

    // Apply decorations
    const oldDecorations = editorRef.current.getModel()?.getAllDecorations()
      .filter(d =>
        d.options.className === 'executing-line-content' ||
        d.options.className === 'passed-line-content' ||
        d.options.className === 'failed-line-content'
      )
      .map(d => d.id) || [];

    editorRef.current.deltaDecorations(oldDecorations, decorations);

  }, [value, monaco, executingLine, isRunning, stepStatuses, stepLinesMap]);

  // Use ref for onFailedStepClick to avoid stale closure
  const onFailedStepClickRef = useRef(onFailedStepClick);
  useEffect(() => {
    onFailedStepClickRef.current = onFailedStepClick;
  }, [onFailedStepClick]);

  const handleEditorDidMount = (editor: editor.IStandaloneCodeEditor, monaco: Monaco) => {
    editorRef.current = editor;
    applyDiagnostics();
    applyPendingReveal();
    editor.addAction({
      id: 'lumi-tester.format-yaml',
      label: 'Format YAML',
      keybindings: [monaco.KeyMod.Shift | monaco.KeyMod.Alt | monaco.KeyCode.KeyF],
      precondition: "editorLangId == yaml",
      run: () => editor.getAction('editor.action.formatDocument')?.run(),
    });
    editor.addAction({
      id: 'lumi-tester.trigger-yaml-suggestions',
      label: 'Trigger Lumi YAML Suggestions',
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.Space],
      precondition: "editorLangId == yaml",
      run: () => editor.trigger('keyboard', 'editor.action.triggerSuggest', null),
    });
    editor.onDidChangeModelContent(() => {
      if (pendingRevealRef.current) requestAnimationFrame(applyPendingReveal);
    });
    requestAnimationFrame(applyPendingReveal);

    // Indent command parameters beneath the command currently being edited.
    editor.onKeyDown((e) => {
      if (e.keyCode !== monaco.KeyCode.Enter) return;
      const model = editor.getModel();
      const position = editor.getPosition();
      if (!model || !position) return;

      const currentLine = model.getLineContent(position.lineNumber);
      const beforeCursor = currentLine.slice(0, position.column - 1);
      const afterCursor = currentLine.slice(position.column - 1);
      const commandMatch = /^(\s*)-\s*[\w-]+:\s*$/.exec(beforeCursor);
      if (!commandMatch || afterCursor.trim()) return;

      e.preventDefault();
      e.stopPropagation();

      const propertyIndent = ' '.repeat(commandMatch[1].length + 4);
      editor.executeEdits('auto-indent-yaml', [{
        range: new monaco.Range(
          position.lineNumber,
          position.column,
          position.lineNumber,
          position.column
        ),
        text: `\n${propertyIndent}`,
      }]);
      editor.setPosition({
        lineNumber: position.lineNumber + 1,
        column: propertyIndent.length + 1,
      });
      requestAnimationFrame(() => editor.trigger('keyboard', 'editor.action.triggerSuggest', null));
    });

    editor.onMouseDown((e) => {
      const model = editor.getModel();
      if (!model) return;

      if (
        e.event.leftButton
        && !e.event.ctrlKey
        && !e.event.metaKey
        && e.target.type === monaco.editor.MouseTargetType.CONTENT_TEXT
        && e.target.position
        && model.getLanguageId() === 'yaml'
      ) {
        const lineNumber = e.target.position.lineNumber;
        const reference = lumiYamlPathAtModel(model, lineNumber, e.target.position.column);
        if (reference) {
          Promise.resolve(onOpenPathRef.current(reference)).catch(error => {
            window.alert(`Could not open referenced file: ${String(error)}`);
          });
        }
      }

      if (e.target.type !== monaco.editor.MouseTargetType.GUTTER_GLYPH_MARGIN) return;
      const lineNumber = e.target.position?.lineNumber;
      const lines = stepLinesMapRef.current;
      const errors = stepErrorsRef.current;
      if (!lineNumber || !lines || !errors) return;

      const zeroBasedLineNumber = lineNumber - 1;
      for (const [stepIndex, stepLine] of lines.entries()) {
        if (stepLine !== zeroBasedLineNumber) continue;
        const error = errors.get(stepIndex);
        if (error) onFailedStepClickRef.current?.(stepIndex, error, zeroBasedLineNumber);
        return;
      }
    });
  };

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    defineCodeverseTheme(monaco);
    registerLumiYamlLanguage(monaco);
    registerLumiYamlFileLinks(monaco, sourcePath, onOpenPath);
    registerLumiYamlCodeLenses(monaco, { runAll: onRunAll, runCommand: onRunCommand, runFromCommand: onRunFromCommand });
    monaco.editor.setTheme('codeverse-dark');

    const instance = monaco.editor.create(container, {
      value,
      language,
      theme: 'codeverse-dark',
      fontFamily: "'Menlo', 'Monaco', 'Courier New', monospace",
      fontSize: 13,
      lineHeight: 20,
      minimap: { enabled: true },
      scrollBeyondLastLine: false,
      glyphMargin: true,
      quickSuggestions: { other: true, comments: false, strings: true },
      quickSuggestionsDelay: 0,
      suggestOnTriggerCharacters: true,
      acceptSuggestionOnCommitCharacter: true,
      acceptSuggestionOnEnter: 'on',
      tabCompletion: 'on',
      wordBasedSuggestions: language === 'yaml' ? 'off' : 'allDocuments',
      suggestSelection: 'first',
      codeLens: true,
      links: true,
      snippetSuggestions: 'top',
      parameterHints: { enabled: true },
      wordWrap: 'on',
      padding: { top: 16, bottom: 100 },
      smoothScrolling: true,
      cursorBlinking: 'blink',
      cursorSmoothCaretAnimation: 'off',
      renderLineHighlight: 'all',
      contextmenu: true,
      bracketPairColorization: { enabled: true },
      guides: { indentation: true, bracketPairs: true },
      autoIndent: 'full',
      tabSize: 2,
      insertSpaces: true,
      automaticLayout: true,
      readOnly,
    });
    editorRef.current = instance;
    handleEditorDidMount(instance, monaco);
    const contentListener = instance.onDidChangeModelContent(() => {
      if (!isApplyingValueRef.current) onChangeRef.current(instance.getValue());
    });

    return () => {
      contentListener.dispose();
      instance.dispose();
      editorRef.current = null;
    };
  }, []);

  useEffect(() => {
    const insertInspectorCommand = (event: Event) => {
      const detail = (event as CustomEvent<{ fileId: string; value: string; inserted: boolean }>).detail;
      const editor = editorRef.current;
      const model = editor?.getModel();
      if (!detail || !editor || !model || sourcePathRef.current !== detail.fileId || model.getLanguageId() !== 'yaml') return;

      const position = editor.getPosition();
      if (!position) return;
      detail.inserted = editor.executeEdits('lumi-inspector', [{
        range: new monaco.Range(position.lineNumber, position.column, position.lineNumber, position.column),
        text: detail.value,
        forceMoveMarkers: true,
      }]);
      if (detail.inserted) editor.focus();
    };

    window.addEventListener('lumi-inspector-insert', insertInspectorCommand);
    return () => window.removeEventListener('lumi-inspector-insert', insertInspectorCommand);
  }, []);

  useEffect(() => {
    const instance = editorRef.current;
    const model = instance?.getModel();
    if (!instance || !model || model.getValue() === value) return;

    isApplyingValueRef.current = true;
    instance.executeEdits('lumi-value-sync', [{
      range: model.getFullModelRange(),
      text: value,
      forceMoveMarkers: true,
    }]);
    isApplyingValueRef.current = false;
  }, [value]);

  useEffect(() => {
    const instance = editorRef.current;
    if (instance) instance.updateOptions({ readOnly });
  }, [readOnly]);

  return (
    <div className="h-full w-full relative overflow-hidden" style={{ backgroundColor: 'var(--ide-canvas)' }}>
      <style>{EXECUTING_CSS}</style>
      <div ref={containerRef} className="h-full w-full" />
    </div>
  );
};
