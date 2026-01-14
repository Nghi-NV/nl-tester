import { Monaco } from "@monaco-editor/react";
import { editor, Position } from "monaco-editor";
import { configCommands, nexusCommands } from './yamlExtension';

// Codeverse Dark Theme
export const defineCodeverseTheme = (monaco: Monaco) => {
  monaco.editor.defineTheme("codeverse-dark", {
    base: "vs-dark",
    inherit: true,
    rules: [
      { token: "", background: "282C34" }, // Atom BG
      { token: "comment", foreground: "5C6370", fontStyle: "italic" }, // Grey
      { token: "keyword", foreground: "C678DD", fontStyle: "bold" }, // Purple
      { token: "string", foreground: "98C379" }, // Green
      { token: "string.key", foreground: "E06C75", fontStyle: "bold" }, // Red for Keys (Atom style)
      { token: "number", foreground: "f472b6" }, // pink-400
      { token: "delimiter", foreground: "94a3b8" }, // slate-400
      { token: "type.identifier", foreground: "c084fc" }, // violet-400
      { token: "attribute.name", foreground: "60a5fa" }, // blue-400
    ],
    colors: {
      "editor.background": "#0f172a", // slate-950
      "editor.foreground": "#e2e8f0", // slate-200
      "editorCursor.foreground": "#22d3ee", // cyan-400
      "editor.lineHighlightBackground": "#1e293b", // slate-800
      "editorLineNumber.foreground": "#475569", // slate-600
      "editorLineNumber.activeForeground": "#22d3ee",
      "editor.selectionBackground": "#22d3ee33", // cyan-400 with opacity
      "editor.inactiveSelectionBackground": "#22d3ee1a",
    },
  });
};

// CSS class for the run button in glyph margin
// This style should be injected globally or via a convenient place
export const RUN_BUTTON_CLASS_NAME = "run-step-glyph";
export const RUN_BUTTON_CSS = `
  .${RUN_BUTTON_CLASS_NAME} {
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='%2310b981' stroke='%2310b981' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolygon points='5 3 19 12 5 21 5 3'/%3E%3C/svg%3E");
    background-size: 12px 12px;
    background-repeat: no-repeat;
    background-position: center;
    cursor: pointer;
    transition: transform 0.2s ease, filter 0.2s ease;
  }
  .${RUN_BUTTON_CLASS_NAME}:hover {
    transform: scale(1.2);
    filter: drop-shadow(0 0 4px rgba(16, 185, 129, 0.5));
  }
`;

export const EXECUTING_CSS = `
  .executing-line-glyph {
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%2322d3ee' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpath d='M21 12a9 9 0 1 1-6.219-8.56'/%3E%3C/svg%3E");
    background-size: 14px 14px;
    background-repeat: no-repeat;
    background-position: center;
    animation: spin 1s linear infinite;
  }
  .executing-line-content {
    background-color: rgba(34, 211, 238, 0.1);
    box-shadow: inset 2px 0 0 0 #22d3ee;
  }
  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
`;

// Helper to create decorations for step lines
export const createStepDecorations = (
  stepLines: Map<number, string>,
  executingLine: number
): editor.IModelDeltaDecoration[] => {
  const decorations: editor.IModelDeltaDecoration[] = [];

  // Add Run Buttons for steps
  stepLines.forEach((stepName, lineIndex) => {
    // Monaco lines are 1-based
    const lineNumber = lineIndex + 1;
    decorations.push({
      range: {
        startLineNumber: lineNumber,
        startColumn: 1,
        endLineNumber: lineNumber,
        endColumn: 1,
      },
      options: {
        isWholeLine: true,
        glyphMarginClassName: RUN_BUTTON_CLASS_NAME,
        glyphMarginHoverMessage: { value: `Run step: ${stepName}` },
      },
    });
  });

  // Add executing line highlight if running
  if (executingLine >= 0) {
    const lineNumber = executingLine + 1;
    decorations.push({
      range: {
        startLineNumber: lineNumber,
        startColumn: 1,
        endLineNumber: lineNumber,
        endColumn: 1,
      },
      options: {
        isWholeLine: true,
        className: "executing-line-content", // Can define CSS for this too
        glyphMarginClassName: "executing-line-glyph", // Optional: spinner?
      },
    });
  }

  return decorations;
};


/**
 * Check if position is in header section (before ---)
 */
function isInHeaderSection(model: { getLineContent: (line: number) => string }, lineNumber: number): boolean {
  for (let i = 1; i < lineNumber; i++) {
    if (model.getLineContent(i).trim() === '---') {
      return false;
    }
  }
  return true;
}

/**
 * Register YAML completion provider with Monaco
 */
export function registerYamlCompletions(monaco: Monaco): void {
  monaco.languages.registerCompletionItemProvider('yaml', {
    triggerCharacters: ['-', ' ', ':', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o', 'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z'],
    provideCompletionItems: (model: editor.ITextModel, position: Position, context: any) => {
      const lineContent = model.getLineContent(position.lineNumber);
      const wordRange = model.getWordUntilPosition(position);
      const currentWord = wordRange.word || '';

      // Get text before cursor
      const beforeCursor = lineContent.substring(0, position.column - 1);
      const trimmedBefore = beforeCursor.trim();

      // Check trigger context
      const isManual = context.triggerKind === monaco.languages.CompletionTriggerKind.Invoke;
      const isTriggerChar = context.triggerKind === monaco.languages.CompletionTriggerKind.TriggerCharacter;
      const isAutomatic = context.triggerKind === monaco.languages.CompletionTriggerKind.Automatic;

      // Only block if line is completely empty and no word and not any trigger
      if (lineContent.trim() === '' && currentWord.length === 0 && !isManual && !isTriggerChar && !isAutomatic) {
        return { suggestions: [] };
      }

      const range = {
        startLineNumber: position.lineNumber,
        endLineNumber: position.lineNumber,
        startColumn: wordRange.startColumn,
        endColumn: wordRange.endColumn,
      };

      const inHeader = isInHeaderSection(model, position.lineNumber);
      const isCommandStart = lineContent.trimStart().startsWith('-') || (trimmedBefore === '' && !inHeader);

      // Combine commands based on context
      const availableCommands = inHeader
        ? [...configCommands, ...nexusCommands]
        : nexusCommands;

      // Always show suggestions, but filter and sort by current word if typing
      let filteredCommands = availableCommands;
      if (currentWord.length > 0) {
        const lowerWord = currentWord.toLowerCase();
        filteredCommands = availableCommands
          .map(cmd => {
            const lowerLabel = cmd.label.toLowerCase();
            const lowerDetail = cmd.detail.toLowerCase();

            // Calculate relevance score - always include all commands but with different scores
            let score = 1; // Default low score
            if (lowerLabel === lowerWord) {
              score = 1000; // Exact match
            } else if (lowerLabel.startsWith(lowerWord)) {
              score = 500; // Starts with
            } else if (lowerLabel.includes(lowerWord)) {
              score = 200; // Contains in label
            } else if (lowerDetail.includes(lowerWord)) {
              score = 100; // Contains in detail
            } else if (cmd.category.toLowerCase().includes(lowerWord)) {
              score = 50; // Contains in category
            }

            return { cmd, score };
          })
          .sort((a, b) => b.score - a.score)
          .map(item => item.cmd);
      }

      const suggestions = filteredCommands.map((cmd, index) => {
        let insertText = cmd.insertText;

        // Auto-add '-' prefix if starting a new step
        if (isCommandStart && !lineContent.includes('-') && !cmd.isConfig) {
          insertText = `- ${insertText}`;
        }

        // Calculate sort text
        let sortText = '';
        if (cmd.isConfig) {
          sortText = '000' + String(index).padStart(3, '0');
        } else if (cmd.category === 'Templates') {
          sortText = '100' + String(index).padStart(3, '0');
        } else if (currentWord.length > 0 && cmd.label.toLowerCase().startsWith(currentWord.toLowerCase())) {
          sortText = '050' + String(index).padStart(3, '0'); // Prioritize starts with
        } else {
          sortText = String(index).padStart(3, '0');
        }

        return {
          label: cmd.label,
          kind: cmd.isConfig
            ? monaco.languages.CompletionItemKind.Property
            : cmd.category === 'Templates'
              ? monaco.languages.CompletionItemKind.Snippet
              : monaco.languages.CompletionItemKind.Function,
          detail: `${cmd.detail} [${cmd.category}]`,
          insertText,
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          range,
          sortText,
        };
      });

      return { suggestions };
    }
  });
}
