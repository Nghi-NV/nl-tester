import React, { useState, useEffect, useMemo, useCallback } from 'react';
import { useFileStore, useEditorStore, useExecutionStore, useDeviceStore, findFile, useExecutionStateStore } from '../stores';
import { Terminal, AlertTriangle, FileJson, X, FolderOpen, Square, Braces, Loader2, CheckCircle2, CircleAlert, Clock3 } from 'lucide-react';
import { clsx } from 'clsx';
import { runTestFlow } from '../services/runnerService';
import { invoke } from '@tauri-apps/api/core';
import { isTauri } from '../utils/tauriUtils';

const EditorCore = React.lazy(async () => {
    await import('./editor/monacoSetup');
    return import('./editor/editorCore').then(module => ({ default: module.EditorCore }));
});

const getEditorLanguage = (filename: string) => {
    const extension = filename.split('.').pop()?.toLowerCase();
    if (extension === 'yaml' || extension === 'yml') return 'yaml';
    if (extension === 'md' || extension === 'markdown') return 'markdown';
    return 'plaintext';
};

interface YamlDiagnostic {
    message: string;
    line?: number;
    column?: number;
}

interface EditorProps {
    onOpenHelp: () => void;
    onOpenProject: () => void;
}

export const Editor: React.FC<EditorProps> = ({ onOpenHelp, onOpenProject }) => {
    const files = useFileStore(state => state.files);
    const projectRoot = useFileStore(state => state.projectRoot);
    const updateFileContent = useFileStore(state => state.updateFileContent);
    const saveFile = useFileStore(state => state.saveFile);
    const saveAllFiles = useFileStore(state => state.saveAllFiles);
    const setFileValidation = useFileStore(state => state.setFileValidation);
    const dirtyFileIds = useFileStore(state => state.dirtyFileIds);
    const openReferencedFile = useFileStore(state => state.openReferencedFile);
    const { activeFileId, openFiles, openFile, closeFile, setActiveView } = useEditorStore();
    const pendingReveal = useEditorStore(state => state.pendingReveal);
    const clearPendingReveal = useEditorStore(state => state.clearPendingReveal);
    const { runningNodeIds, queueRun, stopRun } = useExecutionStore();

    const activeNode = findFile(files, activeFileId);
    const content = activeNode?.content || '';
    const [fileLoadError, setFileLoadError] = useState<{ fileId: string; message: string } | null>(null);
    const [loadAttempt, setLoadAttempt] = useState(0);
    const [runNotice, setRunNotice] = useState<{
        fileId: string;
        state: 'queued' | 'running' | 'passed' | 'failed' | 'cancelled';
        message: string;
    } | null>(null);

    // Load content if missing
    const loadContent = useFileStore(state => state.loadContent);
    useEffect(() => {
        if (!activeNode || activeNode.content !== undefined || activeNode.children) {
            setFileLoadError(null);
            return;
        }

        let cancelled = false;
        let timeoutId: number | undefined;
        setFileLoadError(null);
        const timeout = new Promise<never>((_, reject) => {
            timeoutId = window.setTimeout(() => {
                reject(new Error(`Timed out after 15 seconds while reading ${activeNode.name}.`));
            }, 15_000);
        });
        void Promise.race([loadContent(activeNode.id), timeout])
            .catch(loadError => {
                if (cancelled) return;
                const message = String(loadError);
                setFileLoadError({ fileId: activeNode.id, message });
                setFileValidation(activeNode.id, { state: 'error', message });
            })
            .finally(() => {
                if (timeoutId !== undefined) window.clearTimeout(timeoutId);
            });
        return () => {
            cancelled = true;
            if (timeoutId !== undefined) window.clearTimeout(timeoutId);
        };
    }, [activeNode?.id, activeNode?.content, loadAttempt, loadContent, setFileValidation]);

    const [diagnostics, setDiagnostics] = useState<YamlDiagnostic[]>([]);
    const [formatRequest, setFormatRequest] = useState(0);

    // Validate YAML
    useEffect(() => {
        const isYaml = !!activeNode?.name.match(/\.ya?ml$/i);
        if (!activeFileId || !isYaml) {
            if (activeFileId) setFileValidation(activeFileId);
            setDiagnostics([]);
            return;
        }

        if (activeNode?.content === undefined) {
            setDiagnostics([]);
            return;
        }

        if (!content.trim()) {
            setDiagnostics([]);
            setFileValidation(activeFileId, { state: 'invalid', message: 'File is empty.' });
            return;
        }

        let cancelled = false;
        let validationTimeout: number | undefined;
        setFileValidation(activeFileId, { state: 'checking' });
        const timer = setTimeout(async () => {
            try {
                if (isTauri()) {
                    const validation = await Promise.race([
                        invoke<{ valid: boolean; diagnostics: YamlDiagnostic[] }>('validate_yaml_content', {
                            path: activeFileId,
                            content,
                        }),
                        new Promise<never>((_, reject) => {
                            validationTimeout = window.setTimeout(
                                () => reject(new Error('YAML validation timed out after 15 seconds.')),
                                15_000,
                            );
                        }),
                    ]);
                    if (cancelled) return;
                    setDiagnostics(validation.diagnostics);
                    const message = validation.diagnostics[0]?.message
                        ?? (!validation.valid ? 'YAML validation failed' : undefined);
                    setFileValidation(activeFileId, {
                        state: validation.valid ? 'valid' : 'invalid',
                        message,
                    });
                } else {
                    if (cancelled) return;
                    setDiagnostics([]);
                    setFileValidation(activeFileId, { state: 'valid' });
                }
            } catch (e: any) {
                if (cancelled) return;
                const message = e?.message || String(e);
                const diagnostic = {
                    message,
                    line: 1,
                    column: 1,
                };
                setDiagnostics([diagnostic]);
                setFileValidation(activeFileId, { state: 'error', message: diagnostic.message });
            } finally {
                if (validationTimeout !== undefined) window.clearTimeout(validationTimeout);
            }
        }, 300);

        return () => {
            cancelled = true;
            clearTimeout(timer);
            if (validationTimeout !== undefined) window.clearTimeout(validationTimeout);
        };
    }, [content, activeFileId, activeNode?.content, activeNode?.name, setFileValidation]);

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
        if (!isNodeRunning) return -1;

        // Get executing line from store (using the subscribed value)
        const executingLineFromStore = currentExecutingLine;
        return executingLineFromStore >= 0 ? executingLineFromStore : -1;
    }, [activeFileId, currentExecutingLine, runningNodeIds, activeNode]);

    const handleRunFlow = useCallback(async (commandIndex?: number, fromCommandIndex?: number) => {
        if (!activeNode || !content || !/\.ya?ml$/i.test(activeNode.name)) return;
        const actionLabel = commandIndex !== undefined
            ? `Running command ${commandIndex + 1}…`
            : fromCommandIndex !== undefined
                ? `Running from command ${fromCommandIndex + 1}…`
                : 'Running all commands…';
        const runLabel = `${activeNode.name} · ${actionLabel}`;
        setRunNotice({ fileId: activeNode.id, state: 'queued', message: 'Waiting in run queue…' });

        await queueRun(runLabel, async ({ signal, runId }) => {
          setRunNotice({ fileId: activeNode.id, state: 'running', message: actionLabel });
          useExecutionStore.getState().setNodeRunning(activeNode.id, true);
          try {
            const result = await runTestFlow(
                content,
                activeNode.id,
                activeNode.name,
                useDeviceStore.getState().selectedPlatform,
                useDeviceStore.getState().selectedDevice,
                partial => {
                    if (partial.id) useExecutionStore.getState().upsertResult(partial as any);
                },
                signal,
                { commandIndex, fromCommandIndex, runId },
            );
            useExecutionStore.getState().upsertResult(result);
            if (result.status === 'failed') {
                setRunNotice({
                fileId: activeNode.id,
                state: 'failed',
                message: result.error || (result.failed > 0
                    ? `${result.failed} command${result.failed === 1 ? '' : 's'} failed.`
                    : 'Test run failed.'),
                });
            } else if (result.status === 'cancelled') {
                setRunNotice({ fileId: activeNode.id, state: 'cancelled', message: 'Run cancelled.' });
            } else {
                setRunNotice({
                    fileId: activeNode.id,
                    state: 'passed',
                    message: `${result.passed} command${result.passed === 1 ? '' : 's'} passed.`,
                });
            }
          } catch (runError) {
            setRunNotice({ fileId: activeNode.id, state: 'failed', message: String(runError) });
          } finally {
            useExecutionStore.getState().setNodeRunning(activeNode.id, false);
          }
        });
    }, [activeNode, content, queueRun]);

    const handleRunAll = useCallback(() => { void handleRunFlow(); }, [handleRunFlow]);
    const handleRunCommand = useCallback((index: number) => { void handleRunFlow(index); }, [handleRunFlow]);
    const handleRunFromCommand = useCallback((index: number) => { void handleRunFlow(undefined, index); }, [handleRunFlow]);

    const handleChange = useCallback((newContent: string) => {
        if (!activeNode) return;
        updateFileContent(activeNode.id, newContent);
    }, [activeNode, updateFileContent]);

    const dirtyFileKey = dirtyFileIds.join('\0');
    useEffect(() => {
        if (!dirtyFileIds.length) return;
        const timer = window.setTimeout(() => {
            void saveAllFiles().catch(saveError => {
                if (activeFileId) setFileValidation(activeFileId, { state: 'error', message: String(saveError) });
            });
        }, 600);
        return () => window.clearTimeout(timer);
    }, [dirtyFileKey, content, dirtyFileIds.length, activeFileId, saveAllFiles, setFileValidation]);

    const handleCloseTab = async (fileId: string) => {
        if (dirtyFileIds.includes(fileId)) {
            try {
                await saveFile(fileId);
            } catch (saveError) {
                setFileValidation(fileId, { state: 'error', message: String(saveError) });
                return;
            }
        }
        closeFile(fileId);
    };

    const handleOpenReferencedPath = useCallback(async (reference: string) => {
        if (!activeFileId) return;
        const targetPath = await openReferencedFile(activeFileId, reference);
        openFile(targetPath);
    }, [activeFileId, openFile, openReferencedFile]);

    // Empty state
    if (!openFiles.length) {
        return (
            <div className="ide-welcome flex-1 flex flex-col items-center justify-center text-slate-500">
                <div className="ide-welcome-content">
                    <div className="ide-welcome-icon">
                        <Terminal size={26} />
                    </div>
                    <span className="ide-welcome-eyebrow">LUMI TESTER</span>
                    <h1>{projectRoot ? 'Choose a test flow' : 'Start with a workspace'}</h1>
                    <p>{projectRoot
                        ? 'Open a YAML flow from Explorer to edit steps, inspect selectors, and run the test.'
                        : 'Open a project folder to browse YAML flows, run tests, and review the results in one place.'}</p>
                    <div className="ide-welcome-actions">
                        <button type="button" className="ide-welcome-primary" onClick={onOpenProject}>
                            <FolderOpen size={15} /> Open Folder
                        </button>
                        <button type="button" className="ide-welcome-secondary" onClick={onOpenHelp}>
                            YAML reference
                        </button>
                    </div>
                    <div className="ide-welcome-shortcut"><kbd>⌘</kbd><kbd>⇧</kbd><kbd>P</kbd><span>Search commands</span></div>
                </div>
            </div>
        );
    }

    return (
        <div className="flex-1 flex flex-col bg-slate-950 overflow-hidden">
            {/* Tabs */}
            <div className="ide-editor-tabs flex items-center bg-slate-900 border-b border-slate-800 overflow-hidden shrink-0">
                <div className="ide-editor-tab-strip flex items-stretch flex-1 min-w-0 overflow-x-auto">
                    {openFiles.map(fileId => {
                        const file = findFile(files, fileId);
                        if (!file) return null;
                        const active = activeFileId === fileId;
                        return (
                            <div
                                key={fileId}
                                onClick={() => openFile(fileId)}
                                className={clsx(
                                    "group flex items-center gap-2 px-3 py-1.5 min-w-[120px] max-w-[180px] border-r border-slate-800 cursor-pointer text-xs",
                                    active
                                        ? "bg-slate-950 text-slate-50 border-t-2 border-t-cyan-500"
                                        : "text-slate-400 hover:bg-slate-800 hover:text-slate-200 border-t-2 border-t-transparent"
                                )}
                            >
                                <span className="truncate flex-1">{file.name}</span>
                                {dirtyFileIds.includes(fileId) && (
                                    <span className="w-2 h-2 rounded-full bg-slate-400" title="Unsaved changes" />
                                )}
                                <button
                                    onClick={(e) => { e.stopPropagation(); void handleCloseTab(fileId); }}
                                    className="opacity-0 group-hover:opacity-100 p-0.5 rounded hover:bg-slate-700"
                                    aria-label={`Close ${file.name}`}
                                >
                                    <X size={12} />
                                </button>
                            </div>
                        );
                    })}
                </div>
                <div className="ide-editor-actions flex items-center gap-1 shrink-0 px-2">
                    {activeNode && runningNodeIds.includes(activeNode.id) && (
                        <button
                            type="button"
                            onClick={stopRun}
                            className="ide-editor-icon-button text-rose-400"
                            title="Stop running test"
                            aria-label="Stop running test"
                        >
                            <Square size={13} fill="currentColor" />
                        </button>
                    )}
                    {activeFileId && /\.ya?ml$/i.test(activeNode?.name ?? '') && (
                        <button
                            onClick={() => setFormatRequest(request => request + 1)}
                            className="ide-editor-icon-button"
                            title="Format YAML (Shift+Option+F)"
                            aria-label="Format YAML"
                        >
                            <Braces size={14} />
                        </button>
                    )}
                    <button onClick={onOpenHelp} className="ide-editor-icon-button" title="YAML reference" aria-label="YAML reference">
                        <FileJson size={14} />
                    </button>
                </div>
            </div>

            {runNotice && runNotice.fileId === activeNode?.id && (
                <div
                    className={clsx('ide-run-notice', `is-${runNotice.state}`)}
                    role={runNotice.state === 'failed' ? 'alert' : 'status'}
                    aria-live={runNotice.state === 'failed' ? 'assertive' : 'polite'}
                >
                    <span className="ide-run-notice-icon">
                        {runNotice.state === 'running'
                            ? <Loader2 size={14} className="animate-spin" />
                            : runNotice.state === 'queued'
                                ? <Clock3 size={14} />
                            : runNotice.state === 'passed'
                                ? <CheckCircle2 size={14} />
                                : runNotice.state === 'failed'
                                    ? <CircleAlert size={14} />
                                    : <Square size={12} />}
                    </span>
                    <span className="ide-run-notice-message">{runNotice.message}</span>
                    {runNotice.state === 'running' ? (
                        <button type="button" className="ide-run-notice-action" onClick={stopRun}>Stop</button>
                    ) : (
                        <button type="button" className="ide-run-notice-action" onClick={() => setActiveView('report')}>View report</button>
                    )}
                    <button
                        type="button"
                        className="ide-run-notice-dismiss"
                        aria-label="Dismiss run status"
                        onClick={() => setRunNotice(null)}
                    >
                        <X size={13} />
                    </button>
                </div>
            )}

            {/* Editor Core */}
            {activeNode && (
                <div className="flex-1 overflow-hidden">
                    {activeNode.content === undefined ? (
                        <div className="ide-loading-surface">
                            <div className="flex flex-col items-center gap-3">
                                {fileLoadError?.fileId === activeNode.id ? (
                                    <>
                                        <span>Could not load this file</span>
                                        <span className="max-w-xl text-center text-xs text-rose-400">{fileLoadError.message}</span>
                                        <button
                                            type="button"
                                            className="ide-editor-icon-button"
                                            onClick={() => setLoadAttempt(attempt => attempt + 1)}
                                        >
                                            Retry
                                        </button>
                                    </>
                                ) : <span>Loading file…</span>}
                            </div>
                        </div>
                    ) : (
                        <React.Suspense fallback={<div className="ide-loading-surface">Loading editor…</div>}>
                            <EditorCore
                                value={content}
                                onChange={handleChange}
                                formatRequest={formatRequest}
                                executingLine={executingLine}
                                isRunning={!!activeNode && runningNodeIds.includes(activeNode.id)}
                                stepStatuses={stepStatuses}
                                stepLinesMap={stepLinesMap}
                                stepErrors={stepErrors}
                                onRunAll={handleRunAll}
                                onRunCommand={handleRunCommand}
                                onRunFromCommand={handleRunFromCommand}
                                onFailedStepClick={handleFailedStepClick}
                                language={getEditorLanguage(activeNode.name)}
                                sourcePath={activeFileId}
                                onOpenPath={handleOpenReferencedPath}
                                diagnostics={diagnostics}
                                revealPosition={pendingReveal?.fileId === activeFileId ? pendingReveal : undefined}
                                onRevealComplete={clearPendingReveal}
                            />
                        </React.Suspense>
                    )}
                </div>
            )}

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
