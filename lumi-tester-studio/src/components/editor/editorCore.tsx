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
  // Whether tests are currently running
  isRunning?: boolean;
  // Step statuses map (step index -> status)
  stepStatuses?: Map<number, 'running' | 'passed' | 'failed' | 'pending'>;
  // Step lines map (step index -> line number) from execution state store
  stepLinesMap?: Map<number, number>;
  // Step errors map (step index -> error message)
  stepErrors?: Map<number, string>;
  // Callback when failed icon is clicked
  onFailedStepClick?: (stepIndex: number, error: string, lineNumber: number) => void;
  language?: string;
}

export const EditorCore: React.FC<EditorCoreProps> = ({
  value,
  onChange,
  readOnly = false,
  onRunStep,
  runningSingleStep,
  executingLine = -1,
  isRunning = false,
  stepStatuses,
  stepLinesMap,
  stepErrors,
  onFailedStepClick,
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

    // Only highlight executing line when actually running
    const effectiveExecutingLine = isRunning && executingLine >= 0 ? executingLine : -1;
    const decorations = createStepDecorations(stepLines, effectiveExecutingLine, stepStatuses, stepLinesMap, stepErrors);

    // Apply decorations
    const oldDecorations = editorRef.current.getModel()?.getAllDecorations()
      .filter(d => 
        d.options.glyphMarginClassName?.includes('run-step') || 
        d.options.className === 'executing-line-content' ||
        d.options.className === 'passed-line-content' ||
        d.options.className === 'failed-line-content'
      )
      .map(d => d.id) || [];

    editorRef.current.deltaDecorations(oldDecorations, decorations);

  }, [value, monaco, executingLine, isRunning, runningSingleStep, stepStatuses, stepLinesMap]);

  // Use ref for onFailedStepClick to avoid stale closure
  const onFailedStepClickRef = useRef(onFailedStepClick);
  useEffect(() => {
    onFailedStepClickRef.current = onFailedStepClick;
  }, [onFailedStepClick]);

  const handleEditorDidMount: OnMount = (editor, monaco) => {
    editorRef.current = editor;

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

        // Otherwise, check if it's a run button
        if (stepMapRef.current.has(lineNumber)) {
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
          autoIndent: 'full', // Enable full auto-indent
          tabSize: 2, // Use 2 spaces for tabs
          insertSpaces: true, // Use spaces instead of tabs
          readOnly: readOnly
        }}
      />
    </div>
  );
};
