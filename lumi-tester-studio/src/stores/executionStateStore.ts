import { create } from 'zustand';
import jsyaml from 'js-yaml';
import { TestFlow } from '../types';

export type CommandStatus = 'running' | 'passed' | 'failed' | 'pending';

// Helper function to safely parse YAML (handles both single and multi-document YAML)
export const safeYamlLoad = (content: string): any => {
  try {
    // Try to load as single document first
    return jsyaml.load(content);
  } catch (e: any) {
    // If error is about multiple documents, parse all documents
    if (e.message && e.message.includes('expected a single document')) {
      const documents = jsyaml.loadAll(content);

      if (documents.length === 0) {
        return null;
      }

      // If we have multiple documents, merge them:
      // - Document 1: config/metadata (appId, tags, etc.)
      // - Document 2+: steps array or additional config
      if (documents.length === 1) {
        return documents[0];
      }

      // Merge documents: first doc is config, second doc is steps array
      const config = documents[0] || {};
      const steps = documents[1];

      // If second document is an array, it's the steps
      if (Array.isArray(steps)) {
        return {
          ...config,
          steps: steps
        };
      }

      // Otherwise, merge all documents (if steps is an object)
      if (steps && typeof steps === 'object' && !Array.isArray(steps)) {
        return {
          ...config,
          ...steps
        };
      }

      // If steps is not an object or is an array, just return config
      return config;
    }
    // Re-throw other errors
    throw e;
  }
};

export interface FileExecutionState {
  fileId: string;
  fileName: string;
  // Map of step index -> line number in file
  stepLines: Map<number, number>;
  // Map of step index -> status
  stepStatuses: Map<number, CommandStatus>;
  // Map of step index -> error message (only for failed steps)
  stepErrors: Map<number, string>;
  // Currently executing step index (-1 if none)
  executingStepIndex: number;
  // Currently executing line number (-1 if none)
  executingLine: number;
}

interface ExecutionStateStore {
  // Map of fileId -> FileExecutionState
  fileStates: Map<string, FileExecutionState>;

  // Actions
  startFileExecution: (fileId: string, fileName: string) => void;
  stopFileExecution: (fileId: string) => void;
  setStepLine: (fileId: string, stepIndex: number, lineNumber: number) => void;
  setStepStatus: (fileId: string, stepIndex: number, status: CommandStatus) => void;
  setStepError: (fileId: string, stepIndex: number, error: string) => void;
  setExecutingStep: (fileId: string, stepIndex: number, lineNumber: number) => void;
  clearExecutingStep: (fileId: string) => void;
  clearFileState: (fileId: string) => void;
  clearAllStates: () => void;
  getFileState: (fileId: string) => FileExecutionState | undefined;
  getExecutingLine: (fileId: string) => number;
  // Helper to map steps from YAML content
  mapStepsFromContent: (fileId: string, yamlContent: string) => void;
}

export const useExecutionStateStore = create<ExecutionStateStore>((set, get) => ({
  fileStates: new Map(),

  startFileExecution: (fileId, fileName) => {
    set(state => {
      const newStates = new Map(state.fileStates);
      newStates.set(fileId, {
        fileId,
        fileName,
        stepLines: new Map(),
        stepStatuses: new Map(),
        stepErrors: new Map(),
        executingStepIndex: -1,
        executingLine: -1,
      });
      return { fileStates: newStates };
    });
  },

  stopFileExecution: (fileId) => {
    set(state => {
      const newStates = new Map(state.fileStates);
      const fileState = newStates.get(fileId);
      if (fileState) {
        newStates.set(fileId, {
          ...fileState,
          executingStepIndex: -1,
          executingLine: -1,
        });
      }
      return { fileStates: newStates };
    });
  },

  setStepLine: (fileId, stepIndex, lineNumber) => {
    set(state => {
      const newStates = new Map(state.fileStates);
      const fileState = newStates.get(fileId);
      if (fileState) {
        const newStepLines = new Map(fileState.stepLines);
        newStepLines.set(stepIndex, lineNumber);
        newStates.set(fileId, {
          ...fileState,
          stepLines: newStepLines,
        });
      }
      return { fileStates: newStates };
    });
  },

  setStepStatus: (fileId, stepIndex, status) => {
    set(state => {
      const newStates = new Map(state.fileStates);
      const fileState = newStates.get(fileId);
      if (fileState) {
        const newStepStatuses = new Map(fileState.stepStatuses);
        newStepStatuses.set(stepIndex, status);
        newStates.set(fileId, {
          ...fileState,
          stepStatuses: newStepStatuses,
        });
      }
      return { fileStates: newStates };
    });
  },

  setStepError: (fileId, stepIndex, error) => {
    set(state => {
      const newStates = new Map(state.fileStates);
      const fileState = newStates.get(fileId);
      if (fileState) {
        const newStepErrors = new Map(fileState.stepErrors);
        newStepErrors.set(stepIndex, error);
        newStates.set(fileId, {
          ...fileState,
          stepErrors: newStepErrors,
        });
      }
      return { fileStates: newStates };
    });
  },

  setExecutingStep: (fileId, stepIndex, lineNumber) => {
    set(state => {
      const newStates = new Map(state.fileStates);
      const fileState = newStates.get(fileId);
      if (fileState) {
        newStates.set(fileId, {
          ...fileState,
          executingStepIndex: stepIndex,
          executingLine: lineNumber,
        });
      }
      return { fileStates: newStates };
    });
  },

  clearExecutingStep: (fileId) => {
    set(state => {
      const newStates = new Map(state.fileStates);
      const fileState = newStates.get(fileId);
      if (fileState) {
        newStates.set(fileId, {
          ...fileState,
          executingStepIndex: -1,
          executingLine: -1,
        });
      }
      return { fileStates: newStates };
    });
  },

  clearFileState: (fileId) => {
    set(state => {
      const newStates = new Map(state.fileStates);
      newStates.delete(fileId);
      return { fileStates: newStates };
    });
  },

  clearAllStates: () => {
    set({ fileStates: new Map() });
  },

  getFileState: (fileId) => {
    return get().fileStates.get(fileId);
  },

  getExecutingLine: (fileId) => {
    const fileState = get().fileStates.get(fileId);
    return fileState?.executingLine ?? -1;
  },

  mapStepsFromContent: (fileId, yamlContent) => {
    if (!yamlContent || !fileId) return;

    const lines = yamlContent.split('\n');

    // Find header end (line with '---')
    let headerEndLine = -1;
    for (let i = 0; i < lines.length; i++) {
      if (lines[i].trim() === '---') {
        headerEndLine = i;
        break;
      }
    }

    // Parse YAML to get all steps
    try {
      const parsed = safeYamlLoad(yamlContent) as TestFlow;
      if (!parsed) {
        console.log('[ExecutionStateStore] Failed to parse YAML for mapping steps');
        return;
      }

      // Collect all steps in order (beforeTest, steps, afterTest)
      const allSteps = [
        ...(parsed.beforeTest || []),
        ...(parsed.steps || []),
        ...(parsed.afterTest || [])
      ];

      console.log('[ExecutionStateStore] Mapping steps from content:', {
        fileId,
        totalSteps: allSteps.length,
        beforeTest: parsed.beforeTest?.length ?? 0,
        steps: parsed.steps?.length ?? 0,
        afterTest: parsed.afterTest?.length ?? 0
      });

      // Scan for list items starting from after header
      const listRegex = /^(\s*)-\s/;
      let targetIndent = -1;
      let stepCount = -1;

      for (let i = headerEndLine + 1; i < lines.length; i++) {
        const line = lines[i];
        const listMatch = line.match(listRegex);

        if (listMatch) {
          const indent = listMatch[1].length;

          // First step defines the indentation for top-level steps
          if (targetIndent === -1) {
            targetIndent = indent;
          }

          // Only count steps at the same indentation level
          if (indent === targetIndent) {
            // Check if this is a step by looking for command keys (any key ending with ':')
            // Commands can be: launchApp:, tapOn:, runFlow:, assertVisible:, etc.
            let isStep = false;

            // Check if current line has a command (key ending with ':')
            // Pattern: "- commandName:" or "- commandName: value"
            if (line.match(/^\s*-\s*\w+:\s*/)) {
              isStep = true;
            } else if (line.includes('name:') ||
              line.match(/^\s*-\s*flow:\s*["']?/) ||
              line.match(/^\s*-\s*runFlow:\s*["']?/) ||
              line.match(/^\s*-\s*runFlow:\s*$/)) {
              isStep = true;
            } else {
              // Check next few lines for command indicators
              for (let j = i + 1; j < Math.min(i + 5, lines.length); j++) {
                const nextLine = lines[j];
                const nextListMatch = nextLine.match(/^(\s*)-\s/);
                if (nextListMatch && nextListMatch[1].length <= indent) {
                  break;
                }
                // Check for command key (ending with ':')
                if (nextLine.match(/^\s+\w+:\s*/) ||
                  nextLine.includes('name:') ||
                  nextLine.match(/^\s*flow:\s*["']?/) ||
                  nextLine.match(/^\s*runFlow:\s*["']?/) ||
                  nextLine.match(/^\s*runFlow:\s*$/) ||
                  nextLine.match(/^\s+file:\s*["']?/)) {
                  isStep = true;
                  break;
                }
              }
            }

            if (isStep) {
              stepCount++;
              // Map step index to line number (stepCount is 0-based, matching payload.index)
              get().setStepLine(fileId, stepCount, i);
              console.log('[ExecutionStateStore] Mapped step', stepCount, 'to line', i);
            }
          }
        }
      }

      console.log('[ExecutionStateStore] Finished mapping steps:', {
        fileId,
        mappedSteps: stepCount + 1,
        expectedSteps: allSteps.length
      });
    } catch (e) {
      console.error('[ExecutionStateStore] Failed to map steps from content:', e);
    }
  },
}));
