import React, { useState, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { StepResult } from '../types';
import { X, CheckCircle, XCircle, Clock, AlertCircle, Ban, Terminal, Activity } from 'lucide-react';
import { clsx } from 'clsx';
import Editor, { useMonaco } from '@monaco-editor/react';
import { defineCodeverseTheme } from './editor/monacoUtils';


interface StepDetailModalProps {
  step: StepResult;
  onClose: () => void;
}



export const StepDetailModal: React.FC<StepDetailModalProps> = ({ step, onClose }) => {
  const [activeTab, setActiveTab] = useState<'logs' | 'error'>('logs');

  const monaco = useMonaco();

  // Ensure theme is defined
  useEffect(() => {
    if (monaco) {
      defineCodeverseTheme(monaco);
    }
  }, [monaco]);

  // Close on Escape key
  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };
    window.addEventListener('keydown', handleEscape);
    return () => window.removeEventListener('keydown', handleEscape);
  }, [onClose]);

  const logsText = step.logs ? step.logs.join('\n') : 'No logs available.';

  // Use portal to render outside of any stacking context
  const modalContent = (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 z-[9999] bg-black/60 backdrop-blur-sm animate-in fade-in duration-200"
        onClick={onClose}
      />

      {/* Modal */}
      <div className="fixed inset-0 z-[10000] flex items-center justify-center p-4 pointer-events-none animate-in fade-in duration-200">
        <div
          className="w-full max-w-5xl h-full max-h-[90vh] bg-slate-900 border border-slate-700 rounded-xl shadow-2xl flex flex-col overflow-hidden pointer-events-auto animate-in zoom-in-95 duration-200"
          onClick={(e) => e.stopPropagation()}
        >
          {/* Header */}
          <div className="p-4 border-b border-slate-800 flex items-center justify-between bg-slate-950/50 flex-shrink-0">
            <div className="flex items-center gap-4 min-w-0 flex-1">
              <div className={clsx(
                "p-2 rounded-lg flex-shrink-0",
                step.status === 'passed' ? "bg-emerald-500/10 text-emerald-400" :
                  step.status === 'failed' ? "bg-rose-500/10 text-rose-400" :
                    step.status === 'running' ? "bg-amber-500/10 text-amber-400" : "bg-slate-800 text-slate-400"
              )}>
                {step.status === 'passed' ? <CheckCircle size={20} /> :
                  step.status === 'failed' ? <XCircle size={20} /> :
                    step.status === 'running' ? <Activity size={20} className="animate-pulse" /> : <Ban size={20} />}
              </div>
              <div className="min-w-0 flex-1">
                <h3 className="text-lg font-bold text-slate-100 truncate">{step.name}</h3>
                <div className="flex items-center gap-4 text-xs text-slate-500 mt-1">
                  <span className="flex items-center gap-1">
                    <Clock size={12} />
                    {step.duration ? `${step.duration}ms` : 'Pending'}
                  </span>
                </div>
              </div>
            </div>
            <button
              onClick={onClose}
              className="p-2 hover:bg-slate-800 rounded-lg text-slate-400 hover:text-white transition-colors flex-shrink-0 ml-2"
              aria-label="Close"
            >
              <X size={20} />
            </button>
          </div>

          {/* Tabs */}
          <div className="flex border-b border-slate-800 bg-slate-950/30 flex-shrink-0">
            <button
              onClick={() => setActiveTab('logs')}
              className={clsx(
                "px-6 py-3 text-sm font-bold border-b-2 transition-colors flex items-center gap-2",
                activeTab === 'logs' ? "border-cyan-500 text-cyan-400" : "border-transparent text-slate-500 hover:text-slate-300"
              )}
            >
              <Terminal size={14} />
              <span>Execution Logs</span>
            </button>
            {step.error && (
              <button
                onClick={() => setActiveTab('error')}
                className={clsx(
                  "px-6 py-3 text-sm font-bold border-b-2 transition-colors flex items-center gap-2",
                  activeTab === 'error' ? "border-rose-500 text-rose-400" : "border-transparent text-slate-500 hover:text-rose-400"
                )}
              >
                <AlertCircle size={14} />
                <span>Error Details</span>
              </button>
            )}
          </div>

          {/* Content Area */}
          <div className="flex-1 overflow-hidden bg-slate-950 relative flex flex-col min-h-0">
            {activeTab === 'logs' && (
              <div className="h-full w-full">
                <Editor
                  height="100%"
                  defaultLanguage="text" // or 'log' if custom lang defined
                  theme="codeverse-dark"
                  value={logsText}
                  options={{
                    readOnly: true,
                    minimap: { enabled: false },
                    fontSize: 12,
                    padding: { top: 16, bottom: 16 },
                    scrollBeyondLastLine: false,
                    wordWrap: 'on',
                    fontFamily: 'monospace'
                  }}
                  loading={<div className="text-slate-500 p-4">Loading logs...</div>}
                />
              </div>
            )}

            {activeTab === 'error' && step.error && (
              <div className="p-6 overflow-y-auto h-full">
                <div className="bg-rose-950/20 border border-rose-900/50 rounded-lg p-4">
                  <div className="flex items-start gap-3">
                    <XCircle className="text-rose-500 shrink-0 mt-0.5" size={18} />
                    <div className="flex-1 min-w-0">
                      <h4 className="text-rose-400 font-bold mb-2">Error Message</h4>
                      <pre className="text-rose-300/80 text-sm leading-relaxed whitespace-pre-wrap font-mono break-words bg-rose-950/30 p-3 rounded border border-rose-900/30">
                        {step.error}
                      </pre>
                    </div>
                  </div>
                </div>
              </div>
            )}
          </div>

          {/* Footer */}
          <div className="p-4 bg-slate-900 border-t border-slate-800 flex justify-end flex-shrink-0">
            <button
              onClick={onClose}
              className="px-6 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 text-sm font-medium rounded-lg transition-colors"
            >
              Close
            </button>
          </div>
        </div>
      </div>
    </>
  );

  // Render using portal
  return createPortal(modalContent, document.body);
};
