import { Monaco } from "@monaco-editor/react";
import { editor } from "monaco-editor";
import { configCommands, nexusCommands, commandProperties } from './yamlExtension';

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
  .passed-line-content {
    background-color: rgba(16, 185, 129, 0.1);
    box-shadow: inset 2px 0 0 0 #10b981;
  }
  .failed-line-content {
    background-color: rgba(244, 63, 94, 0.1);
    box-shadow: inset 2px 0 0 0 #f43f5e;
  }
  .passed-line-glyph {
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%2310b981' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cpolyline points='20 6 9 17 4 12'/%3E%3C/svg%3E");
    background-size: 14px 14px;
    background-repeat: no-repeat;
    background-position: center;
  }
  .failed-line-glyph {
    background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%23f43f5e' stroke-width='2' stroke-linecap='round' stroke-linejoin='round'%3E%3Cline x1='18' y1='6' x2='6' y2='18'/%3E%3Cline x1='6' y1='6' x2='18' y2='18'/%3E%3C/svg%3E");
    background-size: 14px 14px;
    background-repeat: no-repeat;
    background-position: center;
    cursor: pointer;
    transition: transform 0.2s ease, filter 0.2s ease;
  }
  .failed-line-glyph:hover {
    transform: scale(1.2);
    filter: drop-shadow(0 0 4px rgba(244, 63, 94, 0.5));
  }
  @keyframes spin {
    from { transform: rotate(0deg); }
    to { transform: rotate(360deg); }
  }
`;

// Helper to create decorations for step lines
export const createStepDecorations = (
  stepLines: Map<number, string>,
  executingLine: number,
  stepStatuses?: Map<number, 'running' | 'passed' | 'failed' | 'pending'>,
  stepLinesMap?: Map<number, number>, // Map from stepIndex to lineNumber (0-based)
  stepErrors?: Map<number, string> // Map from stepIndex to error message
): editor.IModelDeltaDecoration[] => {
  const decorations: editor.IModelDeltaDecoration[] = [];

  // Build a map of line numbers that have passed/failed status to avoid showing run button
  const linesWithStatus = new Set<number>();
  if (stepStatuses && stepLinesMap) {
    stepStatuses.forEach((status, stepIndex) => {
      if (status === 'passed' || status === 'failed') {
        const lineNumber0Based = stepLinesMap.get(stepIndex);
        if (lineNumber0Based !== undefined && lineNumber0Based >= 0) {
          linesWithStatus.add(lineNumber0Based);
        }
      }
    });
  }

  // Add Run Buttons for steps (only if not passed/failed)
  stepLines.forEach((stepName, lineIndex) => {
    // Monaco lines are 1-based
    const lineNumber = lineIndex + 1;
    
    // Skip run button if this line has passed/failed status
    if (linesWithStatus.has(lineIndex)) {
      return;
    }
    
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
        glyphMarginHoverMessage: { value: `Run step: ${stepName} ` },
      },
    });
  });

  // Add executing line highlight if running (do this first so it can be overridden by passed/failed)
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
        className: "executing-line-content",
        glyphMarginClassName: "executing-line-glyph",
      },
    });
  }

  // Add status decorations for each step (passed/failed) - these override executing line
  if (stepStatuses && stepLinesMap) {
    stepStatuses.forEach((status, stepIndex) => {
      // Only show passed/failed, skip running and pending
      if (status !== 'passed' && status !== 'failed') return;
      
      // Get line number from stepLinesMap (0-based)
      const lineNumber0Based = stepLinesMap.get(stepIndex);
      if (lineNumber0Based === undefined || lineNumber0Based < 0) {
        console.warn('[MonacoUtils] Line number not found for step index:', stepIndex, 'status:', status);
        return;
      }
      
      // Convert to 1-based for Monaco
      const lineNumber = lineNumber0Based + 1;

      let className = '';
      let glyphClassName = '';

      if (status === 'passed') {
        className = 'passed-line-content';
        glyphClassName = 'passed-line-glyph';
      } else if (status === 'failed') {
        className = 'failed-line-content';
        glyphClassName = 'failed-line-glyph';
        // Add hover message with error if available
        const error = stepErrors?.get(stepIndex);
        if (error) {
          // Truncate long errors for hover message
          const shortError = error.length > 100 ? error.substring(0, 100) + '...' : error;
          decorations.push({
            range: {
              startLineNumber: lineNumber,
              startColumn: 1,
              endLineNumber: lineNumber,
              endColumn: 1,
            },
            options: {
              isWholeLine: true,
              glyphMarginHoverMessage: { value: `Click to view error details\n\n${shortError}` },
            },
          });
        }
      }

      if (className) {
        decorations.push({
          range: {
            startLineNumber: lineNumber,
            startColumn: 1,
            endLineNumber: lineNumber,
            endColumn: 1,
          },
          options: {
            isWholeLine: true,
            className,
            glyphMarginClassName: glyphClassName,
          },
        });
      }
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
 * Detect if we're inside a command (after command: and on next line with indent)
 */
function detectCommandContext(model: any, position: any): string | null {
  const currentLine = model.getLineContent(position.lineNumber);
  const currentIndent = currentLine.match(/^\s*/)?.[0] || '';
  const currentTrimmed = currentLine.trim();
  
  // Look backwards for the command line
  for (let i = position.lineNumber - 1; i >= 1; i--) {
    const line = model.getLineContent(i);
    const trimmed = line.trim();
    const lineIndent = line.match(/^\s*/)?.[0] || '';
    
    // Skip empty lines
    if (trimmed === '') continue;
    
    // Check if this line has a command with colon (multiline format: - command:)
    const commandMatch = trimmed.match(/^-\s*(\w+):\s*$/);
    if (commandMatch) {
      const commandName = commandMatch[1];
      // Check if current line is indented relative to command line
      // Accept if current line has any indent more than command line (even 1 space, but typically 2+)
      if (currentIndent.length > lineIndent.length) {
        return commandName;
      }
    }
    
    // Check for command: on previous line (most common case)
    // When user types "- swipe:" then presses Enter, we're on the next line
    const inlineCommandMatch = trimmed.match(/^-\s*(\w+):\s*$/);
    if (inlineCommandMatch && i === position.lineNumber - 1) {
      // If the command line ends with just colon, we're likely in its properties
      // Accept if:
      // 1. Current line has any indent more than command line
      // 2. Current line is empty (user just pressed Enter)
      // 3. Current line only has spaces (user is about to type)
      if (currentIndent.length > lineIndent.length || currentTrimmed === '' || (currentTrimmed === '' && currentIndent.length > 0)) {
        return inlineCommandMatch[1];
      }
    }
    
    // Also check for command without dash (if it's a property of another command)
    const propertyCommandMatch = trimmed.match(/^\s*(\w+):\s*$/);
    if (propertyCommandMatch && currentIndent.length > lineIndent.length) {
      // Check if we're inside this property's value
      const propertyName = propertyCommandMatch[1];
      // Only return if we're clearly nested inside (at least 2 spaces more)
      if (currentIndent.length >= lineIndent.length + 2) {
        // Check if the property name matches a command name that has properties
        if (commandProperties[propertyName]) {
          return propertyName;
        }
      }
    }
    
    // If we hit a line with same or less indent and it's not empty, stop looking
    if (lineIndent.length <= currentIndent.length && trimmed !== '' && !trimmed.startsWith('#')) {
      // But allow if it's a command line we just checked
      if (!trimmed.match(/^-\s*(\w+):\s*$/)) {
        break;
      }
    }
    
    // If we hit a line that starts with '-' at same or less indent, stop (unless it's the command we're looking for)
    if (trimmed.startsWith('-') && lineIndent.length <= currentIndent.length) {
      break;
    }
  }
  
  return null;
}

/**
 * Register YAML completion provider with Monaco
 */
export const registerYamlCompletions = (monaco: Monaco) => {
  monaco.languages.registerCompletionItemProvider('yaml', {
    provideCompletionItems: (model: any, position: any, context: any) => {
      // Get word at position
      const wordRange = model.getWordUntilPosition(position);
      const currentWord = wordRange.word;

      // Get line content up to cursor
      const lineContent = model.getLineContent(position.lineNumber);
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

      // Check if we're inside a command (after command: on next line)
      const commandContext = detectCommandContext(model, position);
      
      // If inside a command, provide property completions
      if (commandContext && commandProperties[commandContext]) {
        const properties = commandProperties[commandContext];
        
        // Find the command line to get its indent and format
        let commandIndent = '';
        let hasDash = false;
        let foundCommand = false;
        
        for (let i = position.lineNumber - 1; i >= 1; i--) {
          const line = model.getLineContent(i);
          const trimmed = line.trim();
          const lineIndent = line.match(/^\s*/)?.[0] || '';
          
          // Check for command with dash: "- command:"
          const dashMatch = trimmed.match(/^-\s*(\w+):\s*$/);
          if (dashMatch && commandContext === dashMatch[1]) {
            commandIndent = lineIndent;
            hasDash = true;
            foundCommand = true;
            break;
          }
          // Check for command without dash (nested): "  command:"
          const noDashMatch = trimmed.match(/^\s*(\w+):\s*$/);
          if (noDashMatch && commandContext === noDashMatch[1]) {
            commandIndent = lineIndent;
            hasDash = false;
            foundCommand = true;
            break;
          }
        }
        
        // If we couldn't find command, use current line indent as fallback
        const currentIndent = lineContent.match(/^\s*/)?.[0] || '';
        if (!foundCommand) {
          commandIndent = currentIndent;
          // Assume it has dash if current line starts with dash
          hasDash = lineContent.trimStart().startsWith('-');
        }
        
        // Calculate property indent:
        // - For commands with dash ("- command:"), properties should be indented 4 spaces from start
        //   Example: "- swipe:" -> "    direction: up" (4 spaces from column 0)
        // - For nested commands ("  command:"), properties should be indented 4 spaces from command
        //   Example: "  command:" -> "      property: value" (command indent + 4 spaces)
        // Always use 4 spaces (2 tabs) for properties to make them clearly indented
        const propertyIndent = hasDash ? '    ' : commandIndent + '    ';
        
        console.log('[Completions] Command context:', {
          commandContext,
          commandIndent: `"${commandIndent}"`,
          hasDash,
          propertyIndent: `"${propertyIndent}"`,
          currentLine: lineContent,
          currentIndent: `"${currentIndent}"`
        });
        
        // Filter properties if user is typing
        let filteredProperties = properties;
        if (currentWord.length > 0) {
          const lowerWord = currentWord.toLowerCase();
          filteredProperties = properties.filter(prop => 
            prop.label.toLowerCase().includes(lowerWord) ||
            prop.detail.toLowerCase().includes(lowerWord)
          );
        }
        
        const suggestions = filteredProperties.map((prop, index) => {
          // Add proper indent to insertText
          // Properties should be indented 4 spaces from the command (1 tab = 2 spaces, so 2 tabs = 4 spaces)
          let insertText = prop.insertText;
          
          // If insertText has newlines (multi-line property like object/array), add indent to continuation lines
          if (insertText.includes('\n')) {
            const lines = insertText.split('\n');
            // First line is the property itself (e.g., "property: value")
            // Subsequent lines need extra indent (6 spaces total from command for nested content)
            insertText = propertyIndent + lines[0];
            if (lines.length > 1) {
              // Add extra indent for nested content (property indent + 2 more spaces)
              const nestedIndent = propertyIndent + '  ';
              insertText += '\n' + lines.slice(1).map(line => {
                // If line is already indented in the template, preserve relative indent
                return nestedIndent + line.trimStart();
              }).join('\n');
            }
          } else {
            // Single line property, add indent prefix (4 spaces from command)
            insertText = propertyIndent + insertText;
          }
          
          return {
            label: prop.label,
            kind: monaco.languages.CompletionItemKind.Property,
            detail: prop.detail,
            documentation: prop.documentation || prop.detail,
            insertText: insertText,
            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
            range,
            sortText: String(index).padStart(3, '0'),
          };
        });
        
        return { suggestions };
      }

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
          insertText = `- ${insertText} `;
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
