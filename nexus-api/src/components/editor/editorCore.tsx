import React, { useEffect, useRef } from 'react';
import Editor, { OnMount, useMonaco } from '@monaco-editor/react';
import { editor } from 'monaco-editor';
import { defineCodeverseTheme, createStepDecorations, RUN_BUTTON_CSS, registerYamlCompletions, EXECUTING_CSS } from './monacoUtils';

interface EditorCoreProps {
  value: string;
  onChange: (value: string) => void;
  readOnly?: boolean;
  onRunStep?: (stepName: string) => void;
  // Currently running step name for displaying spinner
  runningSingleStep?: string | null;
  // Executing line number (0-indexed) for debugging highlight
  executingLine?: number;
  language?: string;
}

export const EditorCore: React.FC<EditorCoreProps> = ({
  value,
  onChange,
  readOnly = false,
  onRunStep,
  runningSingleStep,
  executingLine = -1,
  language = 'yaml'
}) => {
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);
  const monaco = useMonaco();

  // Store map of line number (1-based) -> step name for click handling
  const stepMapRef = useRef<Map<number, string>>(new Map());

  // Use ref for onRunStep to avoid stale closure in click handler
  const onRunStepRef = useRef(onRunStep);
  useEffect(() => {
    onRunStepRef.current = onRunStep;
  }, [onRunStep]);

  // Initialize Theme and Completion
  useEffect(() => {
    if (monaco) {
      defineCodeverseTheme(monaco);
      monaco.editor.setTheme('codeverse-dark');

      // Register YAML completions
      registerYamlCompletions(monaco);
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

  // Handle Decorations (Run Buttons & Highlights)
  useEffect(() => {
    if (!editorRef.current || !monaco) return;

    const model = editorRef.current.getModel();
    if (!model) return;

    const lines = value.split('\n');
    const stepLines = new Map<number, string>();

    // Parse steps to map lines
    lines.forEach((line, index) => {
      const match = line.match(/^\s*-\s*name:\s*["']?(.+?)["']?\s*$/);
      if (match) {
        stepLines.set(index, match[1].trim());
      }
    });

    // Update ref for click handler
    stepMapRef.current = new Map();
    stepLines.forEach((name, idx) => stepMapRef.current.set(idx + 1, name));

    const decorations = createStepDecorations(stepLines, executingLine);

    // Apply decorations
    const oldDecorations = editorRef.current.getModel()?.getAllDecorations()
      .filter(d => d.options.glyphMarginClassName?.includes('run-step') || d.options.className === 'executing-line-content')
      .map(d => d.id) || [];

    editorRef.current.deltaDecorations(oldDecorations, decorations);

  }, [value, monaco, executingLine, runningSingleStep]);

  const handleEditorDidMount: OnMount = (editor, monaco) => {
    editorRef.current = editor;

    // Click listener for Glyph Margin (Run Buttons)
    editor.onMouseDown((e) => {
      if (e.target.type === monaco.editor.MouseTargetType.GUTTER_GLYPH_MARGIN) {
        const lineNumber = e.target.position?.lineNumber;
        if (lineNumber && stepMapRef.current.has(lineNumber)) {
          const stepName = stepMapRef.current.get(lineNumber)!;
          // Use ref to get the latest callback
          if (onRunStepRef.current) {
            onRunStepRef.current(stepName);
          }
        }
      }
    });
  };

  const handleEditorChange = (value: string | undefined) => {
    if (value !== undefined) {
      onChange(value);
    }
  };

  return (
    <div className="h-full w-full relative overflow-hidden" style={{ backgroundColor: '#282C34' }}>
      <style>{RUN_BUTTON_CSS} {EXECUTING_CSS}</style>
      <Editor
        height="100%"
        defaultLanguage="yaml"
        language={language}
        theme="codeverse-dark"
        value={value}
        onChange={handleEditorChange}
        onMount={handleEditorDidMount}
        options={{
          fontFamily: "'Menlo', 'Monaco', 'Courier New', monospace",
          fontSize: 13,
          lineHeight: 20,
          minimap: { enabled: true },
          scrollBeyondLastLine: false,
          glyphMargin: true, // Enable for Run buttons
          quickSuggestions: {
            other: true,
            comments: true,
            strings: true
          },
          quickSuggestionsDelay: 100,
          suggestOnTriggerCharacters: true,
          acceptSuggestionOnCommitCharacter: true,
          acceptSuggestionOnEnter: 'on',
          tabCompletion: 'on',
          wordBasedSuggestions: 'allDocuments',
          suggestSelection: 'first',
          snippetSuggestions: 'top',
          parameterHints: {
            enabled: true
          },
          wordWrap: 'on',
          padding: { top: 16, bottom: 100 },
          smoothScrolling: true,
          cursorBlinking: 'blink',
          cursorSmoothCaretAnimation: 'off',
          renderLineHighlight: 'all',
          contextmenu: true,
          bracketPairColorization: { enabled: true },
          guides: {
            indentation: true,
            bracketPairs: true
          },
          readOnly: readOnly
        }}
      />
    </div>
  );
};
