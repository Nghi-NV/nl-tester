import React, { useEffect, useRef, useState } from 'react';
import { useFileStore, useEditorStore, useExecutionStore, useEnvStore, findFile } from '../stores';
import { runTestFlow } from '../services/runnerService';
import { Play, CheckCircle, XCircle, Clock, AlertCircle, Square, Ban, Eye } from 'lucide-react';
import { clsx } from 'clsx';
import { StepDetailModal } from './StepDetailModal';
import { StepResult } from '../types';

export const TestRunner: React.FC = () => {
  const files = useFileStore(state => state.files);
  const activeFileId = useEditorStore(state => state.activeFileId);
  const setActiveView = useEditorStore(state => state.setActiveView);
  const envVars = useEnvStore(state => state.envVars);
  const {
    isRunning, results, addResult,
    startRun: startRunRaw, stopRun: stopRunRaw
  } = useExecutionStore();

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
  const [liveSteps, setLiveSteps] = React.useState<any[]>([]);
  const [selectedStep, setSelectedStep] = useState<StepResult | null>(null);

  // Find the most recent result for the current file
  const lastResult = results.find(r => r.fileId === activeFileId);

  const handleRun = async () => {
    if (!activeNode || !activeNode.content) return;

    const signal = startRun();
    setLiveSteps([]);

    try {
      const result = await runTestFlow(
        activeNode.content,
        envVars,
        activeNode.id,
        activeNode.name,
        (step) => {
          setLiveSteps(prev => [...prev, step]);
        },
        signal
      );

      addResult(result);
    } finally {
      stopRun();
    }
  };

  const handleStop = () => {
    stopRun();
  };

  const scrollRef = useRef<HTMLDivElement>(null);

  // Auto scroll to bottom
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [liveSteps]);

  // Steps to display
  const stepsToDisplay = isRunning ? liveSteps : (lastResult?.steps || []);

  return (
    <>
      <div className="h-full flex flex-col bg-slate-950 border-l border-borderGlass w-96 shadow-2xl z-10">
        {/* Header */}
        <div className="p-4 border-b border-borderGlass flex items-center justify-between bg-slate-900/50 backdrop-blur-sm">
          <h3 className="font-bold text-slate-200">Test Execution</h3>
          {isRunning ? (
            <button
              onClick={handleStop}
              className="flex items-center gap-2 px-4 py-1.5 rounded-full font-semibold text-sm transition-all shadow-lg bg-rose-600 hover:bg-rose-500 text-white hover:shadow-rose-500/20"
            >
              <Square size={14} fill="currentColor" />
              Stop
            </button>
          ) : (
            <button
              onClick={handleRun}
              disabled={!activeNode}
              className={clsx(
                "flex items-center gap-2 px-4 py-1.5 rounded-full font-semibold text-sm transition-all shadow-lg",
                !activeNode
                  ? "bg-slate-800 text-slate-500 cursor-not-allowed"
                  : "bg-cyan-600 hover:bg-cyan-500 text-white hover:shadow-cyan-500/20"
              )}
            >
              <Play size={16} fill="currentColor" />
              Run Flow
            </button>
          )}
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-4 space-y-4" ref={scrollRef}>

          {/* Environment Vars Summary */}
          <div className="bg-slate-900/80 rounded-lg p-3 border border-white/5">
            <h4 className="text-xs font-bold text-slate-500 uppercase mb-2">Active Environment</h4>
            <div className="flex flex-wrap gap-2">
              {envVars.filter(v => v.enabled).map(v => (
                <span key={v.key} className="text-xs px-2 py-1 bg-slate-800 rounded text-cyan-300 border border-white/5">
                  {v.key}
                </span>
              ))}
              {envVars.filter(v => v.enabled).length === 0 && <span className="text-xs text-slate-600 italic">No variables active</span>}
            </div>
          </div>

          {/* Live Steps / Results */}
          <div className="space-y-3">
            {stepsToDisplay.map((step, idx) => {
              const isFlow = step.stepName.startsWith('Flow:');
              return (
                <div
                  key={idx}
                  className={clsx(
                    "bg-slate-900 rounded-lg border border-borderGlass overflow-hidden animate-in fade-in slide-in-from-bottom-2 duration-300 group hover:border-slate-700 transition-colors cursor-pointer",
                    step.depth > 0 && "ml-4 border-l-2 border-l-cyan-500/30"
                  )}
                  style={{ marginLeft: `${(step.depth || 0) * 16}px` }}
                  onClick={() => !isFlow && setSelectedStep(step)}
                >
                  <div className="p-3 flex items-center gap-3">
                    {step.status === 'passed' && <CheckCircle className="text-emerald-500 shrink-0" size={18} />}
                    {step.status === 'failed' && <XCircle className="text-rose-500 shrink-0" size={18} />}
                    {step.status === 'skipped' && <AlertCircle className="text-amber-500 shrink-0" size={18} />}
                    {step.status === 'cancelled' && <Ban className="text-slate-400 shrink-0" size={18} />}

                    <div className="flex-1 min-w-0">
                      <div className="flex items-center justify-between">
                        <p className={clsx("text-sm font-medium truncate", isFlow ? "text-cyan-300" : "text-slate-200")}>
                          {step.stepName}
                        </p>
                        <span className={clsx(
                          "text-xs font-mono px-1.5 rounded",
                          step.responseStatus >= 200 && step.responseStatus < 300 ? "bg-emerald-500/10 text-emerald-400" :
                            step.responseStatus >= 400 ? "bg-rose-500/10 text-rose-400" : "bg-slate-700 text-slate-400"
                        )}>
                          {step.responseStatus || '---'}
                        </span>
                      </div>
                      {!isFlow && (
                        <div className="flex items-center gap-2 mt-1 justify-between">
                          <span className="text-xs text-slate-500 flex items-center gap-1">
                            <Clock size={10} /> {step.responseTime}ms
                          </span>
                          <span className="text-[10px] text-cyan-500 opacity-0 group-hover:opacity-100 transition-opacity flex items-center gap-1">
                            <Eye size={10} /> Details
                          </span>
                        </div>
                      )}
                    </div>
                  </div>

                  {/* Error Message */}
                  {step.error && (
                    <div className="px-3 pb-3">
                      <div className={clsx(
                        "text-xs p-2 rounded border font-mono break-all",
                        step.status === 'cancelled' ? "bg-slate-800 text-slate-400 border-slate-700" : "bg-rose-950/30 text-rose-400 border-rose-500/20"
                      )}>
                        {step.error}
                      </div>
                    </div>
                  )}
                </div>
              )
            })}

            {!isRunning && !lastResult && (
              <div className="text-center py-10 text-slate-600">
                <p>No results yet.</p>
                <p className="text-sm">Press Run to execute the test flow.</p>
              </div>
            )}
          </div>
        </div>

        {/* Footer Link to Report */}
        <div className="p-3 border-t border-borderGlass bg-slate-900/50">
          <button
            onClick={() => setActiveView('report')}
            className="w-full py-2 text-xs font-medium text-slate-400 hover:text-cyan-400 hover:bg-slate-800 rounded transition-colors"
          >
            View Detailed Report & Analytics
          </button>
        </div>
      </div>

      {/* Detail Modal */}
      {selectedStep && (
        <StepDetailModal
          step={selectedStep}
          onClose={() => setSelectedStep(null)}
        />
      )}
    </>
  );
};