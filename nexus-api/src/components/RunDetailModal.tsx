import React from 'react';
import { createPortal } from 'react-dom';
import { TestRunResult, StepResult } from '../types';
import { X, CheckCircle, XCircle, Clock, AlertCircle, Ban, Eye } from 'lucide-react';
import { clsx } from 'clsx';

interface RunDetailModalProps {
  run: TestRunResult;
  onClose: () => void;
  onStepSelect?: (step: StepResult) => void;
}

export const RunDetailModal: React.FC<RunDetailModalProps> = ({ run, onClose, onStepSelect }) => {
  const handleStepClick = (step: StepResult) => {
    if (onStepSelect) {
      onStepSelect(step);
    }
  };

  // Calculate stats
  const passRate = run.steps.length > 0 ? ((run.passed / (run.passed + run.failed)) * 100).toFixed(1) : '0.0';

  // Use portal to render outside of any stacking context
  const modalContent = (
    <>
      <div className="fixed inset-0 z-[100] flex items-center justify-center bg-black/60 backdrop-blur-sm animate-in fade-in duration-200">
        <div className="w-[800px] h-[85vh] bg-slate-950 border border-slate-700 rounded-2xl shadow-2xl flex flex-col overflow-hidden animate-in zoom-in-95 duration-200">

          {/* Header */}
          <div className="p-5 border-b border-white/5 bg-slate-900/50 backdrop-blur flex justify-between items-start">
            <div>
              <div className="flex items-center gap-3 mb-1">
                <h2 className="text-xl font-bold text-slate-100">{run.fileName}</h2>
                <span className="font-mono text-xs text-slate-500 bg-slate-900 px-2 py-0.5 rounded border border-white/5">#{run.id}</span>
              </div>
              <div className="flex items-center gap-4 text-xs text-slate-400">
                <span className="flex items-center gap-1"><Clock size={12} /> {new Date(run.timestamp).toLocaleString()}</span>
                <span>Duration: <span className="text-cyan-400 font-mono">{run.totalDuration}ms</span></span>
              </div>
            </div>

            <div className="flex items-center gap-4">
              <div className="text-right">
                <p className="text-xs text-slate-500 uppercase font-bold">Pass Rate</p>
                <p className={clsx("text-lg font-bold", run.failed === 0 ? "text-emerald-400" : "text-rose-400")}>
                  {passRate}%
                </p>
              </div>
              <button
                onClick={onClose}
                className="p-2 hover:bg-slate-800 rounded-lg text-slate-400 hover:text-white transition-colors"
              >
                <X size={24} />
              </button>
            </div>
          </div>

          {/* Steps List */}
          <div className="flex-1 overflow-y-auto p-5 space-y-3 bg-slate-950/50">
            {run.steps.map((step, idx) => {
              const isFlow = step.stepName.startsWith('Flow:');
              return (
                <div
                  key={idx}
                  className={clsx(
                    "bg-slate-900 rounded-lg border border-white/5 overflow-hidden transition-all",
                    step.depth && step.depth > 0 && "ml-4 border-l-2 border-l-cyan-500/30",
                    !isFlow && "hover:border-slate-600 hover:shadow-lg cursor-pointer group"
                  )}
                  style={{ marginLeft: `${(step.depth || 0) * 16}px` }}
                  onClick={() => !isFlow && handleStepClick(step)}
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
                        <div className="flex items-center justify-between mt-1">
                          <span className="text-xs text-slate-500 flex items-center gap-1">
                            <Clock size={10} /> {step.responseTime}ms
                          </span>
                          <span className="text-[10px] text-cyan-500 opacity-0 group-hover:opacity-100 transition-opacity flex items-center gap-1">
                            <Eye size={10} /> View Details
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
          </div>

          {/* Footer */}
          <div className="p-4 bg-slate-900 border-t border-slate-800 flex justify-end">
            <button
              onClick={onClose}
              className="px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 text-sm font-medium rounded-lg transition-colors"
            >
              Close Report
            </button>
          </div>
        </div>
      </div>
    </>
  );

  // Render using portal to document.body to avoid stacking context issues
  return createPortal(modalContent, document.body);
};
