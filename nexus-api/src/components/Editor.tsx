import React, { useState, useEffect, useMemo, useCallback } from 'react';
import { useFileStore, useEditorStore, useExecutionStore, useEnvStore, findFile } from '../stores';
import { Terminal, AlertTriangle, Check, FileJson, X, Loader2 } from 'lucide-react';
import { clsx } from 'clsx';
import jsyaml from 'js-yaml';
import { TestFlow } from '../types';
import { runTestFlow } from '../services/runnerService';
import { EditorCore } from './editor/editorCore';

interface EditorProps {
    onOpenHelp: () => void;
}

export const Editor: React.FC<EditorProps> = ({ onOpenHelp }) => {
    const files = useFileStore(state => state.files);
    const updateFileContent = useFileStore(state => state.updateFileContent);
    const { activeFileId, openFiles, openFile, closeFile, activeStepName } = useEditorStore();
    const { isRunning, startRun: startRunRaw, stopRun: stopRunRaw, addResult } = useExecutionStore();
    const envVars = useEnvStore(state => state.envVars);

    // Enhanced startRun that also resets activeStepName
    const startRun = () => {
        const signal = startRunRaw();
        useEditorStore.getState().setActiveStepName(null);
        return signal;
    };

    // Enhanced stopRun that also resets activeStepName
    const stopRun = () => {
        stopRunRaw();
        useEditorStore.getState().setActiveStepName(null);
    };

    const activeNode = findFile(files, activeFileId);
    const content = activeNode?.content || '';

    const [error, setError] = useState<string | null>(null);
    const [runningSingleStep, setRunningSingleStep] = useState<string | null>(null);

    // Validate YAML
    useEffect(() => {
        if (!content) {
            setError(null);
            return;
        }

        const timer = setTimeout(() => {
            try {
                jsyaml.load(content);
                setError(null);
            } catch (e: any) {
                setError(e.message?.split('\n')[0] || 'Invalid YAML');
            }
        }, 300);

        return () => clearTimeout(timer);
    }, [content]);

    // Find executing line
    const executingLine = useMemo(() => {
        if (!activeStepName || !content || !isRunning) return -1;
        const lines = content.split('\n');
        const regex = new RegExp(`^\\s*-\\s*name:\\s*["']?${activeStepName.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}["']?`);
        return lines.findIndex(line => regex.test(line));
    }, [activeStepName, content, isRunning]);

    // Run single step
    const handleRunStep = useCallback(async (stepName: string) => {
        if (!activeNode || !content || isRunning) return;

        setRunningSingleStep(stepName);

        try {
            const parsed = jsyaml.load(content) as TestFlow;
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
                    envVars,
                    activeNode.id,
                    `${activeNode.name}:${stepName}`,
                    () => { },
                    signal
                );
                addResult(result);
            } finally {
                stopRun();
            }
        } catch (err) {
            console.error('Failed to run step:', err);
        } finally {
            setRunningSingleStep(null);
        }
    }, [activeNode, content, isRunning, envVars, startRun, stopRun, addResult]);

    // Handle content change
    const handleChange = useCallback((newContent: string) => {
        if (activeNode) {
            updateFileContent(activeNode.id, newContent);
        }
    }, [activeNode, updateFileContent]);

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
                        onRunStep={handleRunStep}
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
        </div>
    );
};