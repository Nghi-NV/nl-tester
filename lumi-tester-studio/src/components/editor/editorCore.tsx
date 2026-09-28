import React, { useEffect, useRef } from 'react';
import type { Monaco } from '@monaco-editor/react';
import * as monacoApi from 'monaco-editor/esm/vs/editor/editor.api.js';
import type { editor } from 'monaco-editor';
import { defineCodeverseTheme, createStepDecorations, EXECUTING_CSS } from './monacoUtils';
import { lumiYamlPathAt, registerLumiYamlCodeLenses, registerLumiYamlFileLinks, registerLumiYamlLanguage } from './lumiYamlLanguage';

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
  const valueRef = useRef(value);
  const onChangeRef = useRef(onChange);
  const onOpenPathRef = useRef(onOpenPath);
  const monaco = monacoApi as Monaco;

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
    const model = editorRef.current?.getModel();
    if (!monaco || !model) return;
    monaco.editor.setModelMarkers(model, 'lumi-tester', diagnostics.map(diagnostic => ({
      severity: monaco.MarkerSeverity.Error,
      message: diagnostic.message,
      startLineNumber: diagnostic.line ?? 1,
      endLineNumber: diagnostic.line ?? 1,
      startColumn: diagnostic.column ?? 1,
      endColumn: (diagnostic.column ?? 1) + 1,
      source: 'Lumi Tester',
    })));
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

    // Handle Enter key to auto-indent for YAML commands
    // Use onKeyDown to intercept Enter and handle it ourselves
    editor.onKeyDown((e) => {
      if (e.keyCode === monaco.KeyCode.Enter) {
        const model = editor.getModel();
        if (!model) return;

        const position = editor.getPosition();
        if (!position || position.lineNumber < 2) return;

        // Check if previous line ends with command: (e.g., "- swipe:")
        const prevLine = model.getLineContent(position.lineNumber - 1);
        const prevTrimmed = prevLine.trim();
        
        // Match pattern: "- command:" (ends with colon and optional spaces)
        const commandMatch = prevTrimmed.match(/^-\s*(\w+):\s*$/);
        if (commandMatch) {
          // Get current line content before Enter
          const currentLine = model.getLineContent(position.lineNumber);
          const beforeCursor = currentLine.substring(0, position.column - 1);
          const afterCursor = currentLine.substring(position.column - 1);
          
          // Only handle if cursor is at end of line or line is empty
          if (afterCursor.trim() === '' || currentLine.trim() === '') {
            // Prevent default Enter behavior
            e.preventDefault();
            e.stopPropagation();
            
            // Calculate property indent: 4 spaces from start (2 tabs)
            const propertyIndent = '    ';
            
            console.log('[Auto-Indent] Intercepting Enter for command:', {
              command: commandMatch[1],
              prevLine,
              currentLine,
              beforeCursor,
              afterCursor,
              position
            });
            
            // Insert newline with proper indent
            const newLine = '\n' + propertyIndent;
            
            editor.executeEdits('auto-indent-yaml', [{
              range: new monaco.Range(
                position.lineNumber,
                position.column,
                position.lineNumber,
                position.column
              ),
              text: newLine,
            }]);
            
            // Move cursor to end of indent
            setTimeout(() => {
              const newPosition = new monaco.Position(
                position.lineNumber + 1,
                propertyIndent.length + 1
              );
              editor.setPosition(newPosition);
              console.log('[Auto-Indent] Completed, cursor at:', newPosition);
            }, 0);
          }
        }
      }
    });

    // Click listener for Glyph Margin (Run Buttons and Failed Icons)
    editor.onMouseDown((e) => {
      if (
        e.event.leftButton
        && !e.event.ctrlKey
        && !e.event.metaKey
        && e.target.type === monaco.editor.MouseTargetType.CONTENT_TEXT
        && e.target.position
        && editor.getModel()?.getLanguageId() === 'yaml'
      ) {
        const reference = lumiYamlPathAt(
          editor.getModel()!.getLineContent(e.target.position.lineNumber),
          e.target.position.column,
        );
        if (reference) {
          Promise.resolve(onOpenPathRef.current(reference)).catch(error => {
            window.alert(`Could not open referenced file: ${String(error)}`);
          });
        }
      }

      if (e.target.type === monaco.editor.MouseTargetType.GUTTER_GLYPH_MARGIN) {
        const lineNumber = e.target.position?.lineNumber;
        if (!lineNumber) return;

        // Check if this is a failed step
        if (stepErrors && stepLinesMap) {
          // Find step index for this line (convert 1-based to 0-based)
          const lineNumber0Based = lineNumber - 1;
          for (const [stepIndex, stepLine] of stepLinesMap.entries()) {
            if (stepLine === lineNumber0Based && stepErrors.has(stepIndex)) {
              const error = stepErrors.get(stepIndex)!;
              if (onFailedStepClickRef.current) {
                onFailedStepClickRef.current(stepIndex, error, lineNumber0Based);
              }
              return;
            }
          }
        }

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
      quickSuggestions: { other: true, comments: true, strings: true },
      quickSuggestionsDelay: 100,
      suggestOnTriggerCharacters: true,
      acceptSuggestionOnCommitCharacter: true,
      acceptSuggestionOnEnter: 'on',
      tabCompletion: 'on',
      wordBasedSuggestions: 'allDocuments',
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
