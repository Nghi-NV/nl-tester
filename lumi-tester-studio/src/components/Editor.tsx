import React, { useState, useEffect, useMemo, useCallback } from 'react';
import { useFileStore, useEditorStore, useExecutionStore, useEnvStore, useDeviceStore, findFile, useExecutionStateStore, safeYamlLoad } from '../stores';
import { Terminal, AlertTriangle, Check, FileJson, X, Loader2, Square } from 'lucide-react';
import { clsx } from 'clsx';
import jsyaml from 'js-yaml';
import { TestFlow } from '../types';
import { runTestFlow } from '../services/runnerService';
import { EditorCore } from './editor/editorCore';
import { updateFileContentInTree } from '../utils/treeUtils';

interface EditorProps {
    onOpenHelp: () => void;
}

export const Editor: React.FC<EditorProps> = ({ onOpenHelp }) => {
    const files = useFileStore(state => state.files);
    const updateFileContent = useFileStore(state => state.updateFileContent);
    const { activeFileId, openFiles, openFile, closeFile, setActiveView } = useEditorStore();
    const { isRunning, runningNodeIds, startRun: startRunRaw, stopRun: stopRunRaw, addResult } = useExecutionStore();
    const envVars = useEnvStore(state => state.envVars);

    // Enhanced startRun
    const startRun = () => {
        const signal = startRunRaw();
        return signal;
    };

    // Enhanced stopRun - stop all and clear execution states
    const stopRun = () => {
        stopRunRaw();
        // Clear all execution states when stopping
        useExecutionStateStore.getState().clearAllStates();
    };

    const activeNode = findFile(files, activeFileId);
    const content = activeNode?.content || '';

    // Load content if missing
    const loadContent = useFileStore(state => state.loadContent);
    useEffect(() => {
        if (activeNode && activeNode.content === undefined && !activeNode.children) {
            loadContent(activeNode.id);
        }
    }, [activeNode?.id, activeNode?.content, loadContent]);

    const [error, setError] = useState<string | null>(null);
    const [runningSingleStep, setRunningSingleStep] = useState<string | null>(null);

    // Helper function to map step indices to line numbers
    const mapStepsToLines = useCallback((fileId: string, content: string) => {
        if (!content || !fileId) return;

        const lines = content.split('\n');

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
            const parsed = safeYamlLoad(content) as TestFlow;
            if (!parsed) {
                console.log('[Editor] Failed to parse YAML for mapping steps');
                return;
            }

            // Collect all steps in order (beforeTest, steps, afterTest)
            const allSteps = [
                ...(parsed.beforeTest || []),
                ...(parsed.steps || []),
                ...(parsed.afterTest || [])
            ];

            console.log('[Editor] Mapping steps to lines:', {
                fileId,
                totalSteps: allSteps.length,
                beforeTest: parsed.beforeTest?.length ?? 0,
                steps: parsed.steps?.length ?? 0,
                afterTest: parsed.afterTest?.length ?? 0
            });

            // Get store state directly to avoid dependency issues
            const stateStore = useExecutionStateStore.getState();

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
                            stateStore.setStepLine(fileId, stepCount, i);
                            console.log('[Editor] Mapped step', stepCount, 'to line', i);
                        }
                    }
                }
            }

            console.log('[Editor] Finished mapping steps:', {
                fileId,
                mappedSteps: stepCount + 1,
                expectedSteps: allSteps.length
            });
        } catch (e) {
            console.error('[Editor] Failed to map steps to lines:', e);
        }
    }, []);

    // Map steps to lines when content changes
    useEffect(() => {
        if (activeFileId && content) {
            mapStepsToLines(activeFileId, content);
        }
    }, [activeFileId, content, mapStepsToLines]);

    // Validate YAML
    useEffect(() => {
        if (!content) {
            setError(null);
            return;
        }

        const timer = setTimeout(() => {
            try {
                safeYamlLoad(content);
                setError(null);
            } catch (e: any) {
                setError(e.message?.split('\n')[0] || 'Invalid YAML');
            }
        }, 300);

        return () => clearTimeout(timer);
    }, [content]);

    // Subscribe to execution state store changes - only specific values to avoid infinite loop
    const executingStepIndex = useExecutionStateStore(state => {
        const fileState = activeFileId ? state.fileStates.get(activeFileId) : undefined;
        return fileState?.executingStepIndex ?? -1;
    });
    const stepLinesSize = useExecutionStateStore(state => {
        const fileState = activeFileId ? state.fileStates.get(activeFileId) : undefined;
        return fileState?.stepLines.size ?? 0;
    });
    const currentExecutingLine = useExecutionStateStore(state => {
        const fileState = activeFileId ? state.fileStates.get(activeFileId) : undefined;
        return fileState?.executingLine ?? -1;
    });
    const stepStatuses = useExecutionStateStore(state => {
        const fileState = activeFileId ? state.fileStates.get(activeFileId) : undefined;
        return fileState?.stepStatuses;
    });
    const stepLinesMap = useExecutionStateStore(state => {
        const fileState = activeFileId ? state.fileStates.get(activeFileId) : undefined;
        return fileState?.stepLines;
    });
    const stepErrors = useExecutionStateStore(state => {
        const fileState = activeFileId ? state.fileStates.get(activeFileId) : undefined;
        return fileState?.stepErrors;
    });

    // State for error modal
    const [errorModal, setErrorModal] = useState<{ stepIndex: number; error: string; lineNumber: number } | null>(null);

    // Handler for failed step click
    const handleFailedStepClick = useCallback((stepIndex: number, error: string, lineNumber: number) => {
        setErrorModal({ stepIndex, error, lineNumber });
    }, []);

    // Update executing line when step status changes
    useEffect(() => {
        if (!activeFileId || !content || executingStepIndex < 0) return;

        // Get stepLinesMap from store when needed
        const stateStore = useExecutionStateStore.getState();
        const fileState = stateStore.getFileState(activeFileId);
        if (!fileState) return;

        const lineNumber = fileState.stepLines.get(executingStepIndex);
        console.log('[Editor] Updating executing line:', {
            fileId: activeFileId,
            executingStepIndex,
            lineNumber,
            currentExecutingLine,
            stepLinesSize: fileState.stepLines.size
        });

        // Only update if line number is found and different from current
        if (lineNumber !== undefined && lineNumber !== currentExecutingLine) {
            console.log('[Editor] Setting executing step with line number:', lineNumber);
            stateStore.setExecutingStep(activeFileId, executingStepIndex, lineNumber);
        } else if (lineNumber === undefined) {
            console.warn('[Editor] Line number not found for step index:', executingStepIndex, 'Available step lines:', Array.from(fileState.stepLines.entries()));
        }
    }, [activeFileId, content, executingStepIndex, currentExecutingLine, stepLinesSize]);

    // Get executing line from execution state store - only when actually running
    const executingLine = useMemo(() => {
        if (!activeFileId) return -1;

        // Only highlight if actually running
        const isNodeRunning = activeNode && runningNodeIds.includes(activeNode.id);
        if (!isRunning && !isNodeRunning) return -1;

        // Get executing line from store (using the subscribed value)
        const executingLineFromStore = currentExecutingLine;
        return executingLineFromStore >= 0 ? executingLineFromStore : -1;
    }, [activeFileId, currentExecutingLine, isRunning, runningNodeIds, activeNode]);

    // Run single step
    const handleRunStep = useCallback(async (stepName: string) => {
        if (!activeNode || !content || isRunning) return;

        setRunningSingleStep(stepName);

        try {
            const parsed = safeYamlLoad(content) as TestFlow;
            if (!parsed) return;

            const allSteps = [
                ...(parsed.beforeTest || []),
                ...(parsed.steps || []),
                ...(parsed.afterTest || [])
            ];

            // Normalize step name - remove surrounding quotes and trim
            const normalizeStepName = (name: string) => {
                if (!name) return '';
                let normalized = name.trim();
                // Remove surrounding quotes if present
                if ((normalized.startsWith('"') && normalized.endsWith('"')) ||
                    (normalized.startsWith("'") && normalized.endsWith("'"))) {
                    normalized = normalized.slice(1, -1);
                }
                return normalized.trim();
            };

            const normalizedSearchName = normalizeStepName(stepName);
            const step = allSteps.find(s => normalizeStepName(s.name) === normalizedSearchName);
            if (!step) return;

            const singleStepFlow: TestFlow = {
                name: `Run: ${stepName}`,
                config: parsed.config,
                steps: [step]
            };

            const yamlContent = jsyaml.dump(singleStepFlow);
            const signal = startRun();

            try {
                const result = await runTestFlow(
                    yamlContent,

                    activeNode.id,
                    `${activeNode.name}: ${stepName}`,
                    useDeviceStore.getState().selectedPlatform,
                    useDeviceStore.getState().selectedDevice,
                    (partial) => {
                        if (partial.id) {
                            useExecutionStore.getState().upsertResult(partial as any);
                        }
                    },
                    signal
                );
                useExecutionStore.getState().upsertResult(result);
                
                // Tự động chuyển sang tab Reports sau khi test chạy xong
                setActiveView('report');
            } finally {
                stopRun();
            }
        } catch (err) {
            console.error('Failed to run step:', err);
        } finally {
            setRunningSingleStep(null);
        }
    }, [activeNode, content, isRunning, envVars, startRun, stopRun, addResult, setActiveView]);

    // Debounced save function
    const saveTimeoutRef = React.useRef<number | null>(null);
    const pendingContentRef = React.useRef<string | null>(null);

    // Handle content change with debounce
    const handleChange = useCallback((newContent: string) => {
        if (!activeNode) return;

        // Update content in memory immediately for responsive UI (without saving to disk)
        const stateStore = useFileStore.getState();
        stateStore.files = updateFileContentInTree(stateStore.files, activeNode.id, newContent);
        useFileStore.setState({ files: stateStore.files });

        // Store pending content for debounced save
        pendingContentRef.current = newContent;

        // Clear previous timeout
        if (saveTimeoutRef.current) {
            clearTimeout(saveTimeoutRef.current);
        }

        // Debounce: Save to disk after 500ms of no changes
        saveTimeoutRef.current = setTimeout(() => {
            if (pendingContentRef.current && activeNode) {
                updateFileContent(activeNode.id, pendingContentRef.current).catch(err => {
                    console.error('Failed to save file (debounced):', err);
                });
                pendingContentRef.current = null;
            }
        }, 500);
    }, [activeNode, updateFileContent]);

    // Cleanup timeout on unmount or file change, and save pending changes
    useEffect(() => {
        return () => {
            if (saveTimeoutRef.current) {
                clearTimeout(saveTimeoutRef.current);
            }
            // Save any pending changes before switching files
            if (pendingContentRef.current && activeNode) {
                updateFileContent(activeNode.id, pendingContentRef.current).catch(err => {
                    console.error('Failed to save file (cleanup):', err);
                });
                pendingContentRef.current = null;
            }
        };
    }, [activeFileId, activeNode, updateFileContent]);

    // Line count for display
    const lineCount = content.split('\n').length;

    // Empty state
    if (!openFiles.length) {
        return (
            <div className="flex-1 flex flex-col items-center justify-center bg-slate-950 text-slate-500">
                <div className="w-20 h-20 bg-gradient-to-br from-slate-800 to-slate-900 rounded-2xl flex items-center justify-center mb-4">
                    <Terminal size={36} className="text-slate-600" />
                </div>
                <p className="text-lg font-medium">No files open</p>
                <p className="text-sm mt-2 text-slate-600">Select a file from the explorer</p>
            </div>
        );
    }

    return (
        <div className="flex-1 flex flex-col bg-slate-950 overflow-hidden">
            {/* Tabs */}
            <div className="flex items-center bg-slate-950 border-b border-slate-800 overflow-x-auto shrink-0">
                {openFiles.map(fileId => {
                    const file = findFile(files, fileId);
                    if (!file) return null;
                    const active = activeFileId === fileId;
                    return (
                        <div
                            key={fileId}
                            onClick={() => openFile(fileId)}
                            className={clsx(
                                "group flex items-center gap-2 px-4 py-2.5 min-w-[120px] max-w-[180px] border-r border-slate-800 cursor-pointer text-sm",
                                active
                                    ? "bg-slate-900 text-cyan-400 border-t-2 border-t-cyan-500"
                                    : "text-slate-500 hover:bg-slate-900/50 hover:text-slate-300 border-t-2 border-t-transparent"
                            )}
                        >
                            <span className="truncate flex-1">{file.name}</span>
                            <button
                                onClick={(e) => { e.stopPropagation(); closeFile(fileId); }}
                                className="opacity-0 group-hover:opacity-100 p-0.5 rounded hover:bg-slate-700"
                            >
                                <X size={12} />
                            </button>
                        </div>
                    );
                })}
            </div>

            {/* Toolbar */}
            <div className="h-9 border-b border-slate-800 bg-slate-900/50 flex items-center justify-between px-4 shrink-0">
                <div className="flex items-center gap-3 text-xs text-slate-500">
                    <span className="font-mono bg-slate-800 px-2 py-0.5 rounded">
                        {lineCount} lines
                    </span>
                    {isRunning && (
                        <span className="flex items-center gap-1 text-cyan-400 animate-pulse">
                            <Loader2 size={12} className="animate-spin" />
                            Running...
                        </span>
                    )}
                </div>
                <div className="flex items-center gap-2">
                    {isRunning && (
                        <button
                            onClick={stopRun}
                            className="flex items-center gap-1.5 px-3 py-1 text-xs font-medium text-rose-400 hover:text-rose-300 bg-rose-950/30 hover:bg-rose-950/50 rounded transition-colors"
                            title="Stop running tests"
                        >
                            <Square size={12} fill="currentColor" />
                            Stop
                        </button>
                    )}
                    {error ? (
                        <div className="flex items-center gap-1 text-rose-400 text-xs px-2 py-0.5 bg-rose-950/30 rounded">
                            <AlertTriangle size={11} />
                            <span className="max-w-[180px] truncate">{error}</span>
                        </div>
                    ) : (
                        <div className="flex items-center gap-1 text-emerald-400 text-xs px-2 py-0.5 bg-emerald-950/30 rounded">
                            <Check size={11} />
                            Valid
                        </div>
                    )}
                    <button onClick={onOpenHelp} className="text-slate-500 hover:text-cyan-400 p-1">
                        <FileJson size={14} />
                    </button>
                </div>
            </div>

            {/* Editor Core */}
            {activeNode && (
                <div className="flex-1 overflow-hidden">
                    <EditorCore
                        value={content}
                        onChange={handleChange}
                        executingLine={executingLine}
                        isRunning={isRunning || (activeNode && runningNodeIds.includes(activeNode.id))}
                        stepStatuses={stepStatuses}
                        stepLinesMap={stepLinesMap}
                        stepErrors={stepErrors}
                        onRunStep={handleRunStep}
                        onFailedStepClick={handleFailedStepClick}
                        runningSingleStep={runningSingleStep}
                        language={activeNode.name.endsWith('.md') ? 'markdown' : 'yaml'}
                    />
                </div>
            )}

            {/* Shortcuts Bar */}
            <div className="h-6 bg-slate-900/50 border-t border-slate-800 flex items-center justify-center gap-6 text-[10px] text-slate-600 shrink-0">
                <span><span className="text-slate-500">⌘D</span> Duplicate</span>
                <span><span className="text-slate-500">⌘/</span> Comment</span>
                <span><span className="text-slate-500">Tab</span> Indent</span>
            </div>

            {/* Error Modal */}
            {errorModal && (
                <div className="fixed inset-0 bg-black/50 flex items-center justify-center z-50" onClick={() => setErrorModal(null)}>
                    <div className="bg-slate-900 border border-slate-700 rounded-lg shadow-xl max-w-2xl w-full mx-4 max-h-[80vh] flex flex-col" onClick={(e) => e.stopPropagation()}>
                        <div className="flex items-center justify-between p-4 border-b border-slate-700">
                            <div className="flex items-center gap-2">
                                <AlertTriangle className="text-red-500" size={20} />
                                <h3 className="text-lg font-semibold text-slate-200">Test Step Failed</h3>
                            </div>
                            <button
                                onClick={() => setErrorModal(null)}
                                className="text-slate-400 hover:text-slate-200 p-1 rounded hover:bg-slate-800"
                            >
                                <X size={18} />
                            </button>
                        </div>
                        <div className="p-4 overflow-y-auto flex-1">
                            <div className="mb-4">
                                <p className="text-sm text-slate-400 mb-1">Step Index:</p>
                                <p className="text-slate-200 font-mono">{errorModal.stepIndex}</p>
                            </div>
                            <div className="mb-4">
                                <p className="text-sm text-slate-400 mb-1">Line Number:</p>
                                <p className="text-slate-200 font-mono">{errorModal.lineNumber + 1}</p>
                            </div>
                            <div>
                                <p className="text-sm text-slate-400 mb-2">Error Message:</p>
                                <pre className="bg-slate-950 border border-slate-800 rounded p-3 text-sm text-red-400 font-mono whitespace-pre-wrap overflow-x-auto">
                                    {errorModal.error}
                                </pre>
                            </div>
                        </div>
                        <div className="p-4 border-t border-slate-700 flex justify-end">
                            <button
                                onClick={() => setErrorModal(null)}
                                className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 rounded transition-colors"
                            >
                                Close
                            </button>
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
};
