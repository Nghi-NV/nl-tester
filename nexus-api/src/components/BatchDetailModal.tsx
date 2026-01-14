import React, { useState } from 'react';
import { createPortal } from 'react-dom';
import { TestRunResult, StepResult } from '../types';
import { X, Clock, CheckCircle, XCircle, ArrowRight, Folder } from 'lucide-react';
import { RunDetailModal } from './RunDetailModal';

interface BatchDetailModalProps {
  batchId: string;
  runs: TestRunResult[];
  onClose: () => void;
  onStepSelect?: (step: StepResult) => void;
}

export const BatchDetailModal: React.FC<BatchDetailModalProps> = ({ batchId, runs, onClose, onStepSelect }) => {
  const [selectedRun, setSelectedRun] = useState<TestRunResult | null>(null);

  // Stats
  const totalFiles = runs.length;
  const passedFiles = runs.filter(r => r.failed === 0).length;
  const failedFiles = runs.filter(r => r.failed > 0).length;
  const totalDuration = runs.reduce((acc, r) => acc + r.totalDuration, 0);

  // First run timestamp as batch timestamp
  const timestamp = runs.length > 0 ? runs[0].timestamp : Date.now();
  const folderName = runs.length > 0 ? runs[0].folderName : 'Unknown Folder';

  // Use portal to render outside of any stacking context
  const modalContent = (
    <>
      <div className="fixed inset-0 z-[100] flex items-center justify-center bg-black/60 backdrop-blur-sm animate-in fade-in duration-200">
        <div className="w-[800px] h-[80vh] bg-slate-950 border border-slate-700 rounded-2xl shadow-2xl flex flex-col overflow-hidden animate-in zoom-in-95 duration-200">

          {/* Header */}
          <div className="p-5 border-b border-white/5 bg-slate-900/50 backdrop-blur flex justify-between items-start">
            <div>
              <h2 className="text-xl font-bold text-slate-100 flex items-center gap-2">
                <Folder className="text-cyan-500" />
                {folderName}
              </h2>
              <div className="flex items-center gap-4 text-xs text-slate-400 mt-2">
                <span className="font-mono bg-slate-800 px-2 py-0.5 rounded">ID: {batchId}</span>
                <span className="flex items-center gap-1"><Clock size={12} /> {new Date(timestamp).toLocaleString()}</span>
              </div>
            </div>
            <button onClick={onClose} className="p-2 hover:bg-slate-800 rounded-lg text-slate-400 hover:text-white transition-colors">
              <X size={24} />
            </button>
          </div>

          {/* KPI Summary */}
          <div className="grid grid-cols-4 gap-4 p-5 pb-0">
            <div className="bg-slate-900 rounded-lg p-3 border border-slate-800">
              <p className="text-xs text-slate-500 uppercase">Total Files</p>
              <p className="text-xl font-bold text-white">{totalFiles}</p>
            </div>
            <div className="bg-slate-900 rounded-lg p-3 border border-slate-800">
              <p className="text-xs text-slate-500 uppercase">Passed Files</p>
              <p className="text-xl font-bold text-emerald-400">{passedFiles}</p>
            </div>
            <div className="bg-slate-900 rounded-lg p-3 border border-slate-800">
              <p className="text-xs text-slate-500 uppercase">Failed Files</p>
              <p className="text-xl font-bold text-rose-400">{failedFiles}</p>
            </div>
            <div className="bg-slate-900 rounded-lg p-3 border border-slate-800">
              <p className="text-xs text-slate-500 uppercase">Duration</p>
              <p className="text-xl font-bold text-cyan-400">{totalDuration}ms</p>
            </div>
          </div>

          {/* List */}
          <div className="flex-1 overflow-y-auto p-5 space-y-3">
            <h3 className="text-sm font-semibold text-slate-300 mb-2">Executed Files</h3>
            {runs.map((run) => (
              <div
                key={run.id}
                onClick={() => setSelectedRun(run)}
                className="bg-slate-900/50 hover:bg-slate-800 border border-white/5 rounded-lg p-3 flex items-center justify-between cursor-pointer transition-colors group"
              >
                <div className="flex items-center gap-3">
                  {run.failed === 0 ? <CheckCircle size={18} className="text-emerald-500" /> : <XCircle size={18} className="text-rose-500" />}
                  <div>
                    <p className="text-sm font-medium text-slate-200 group-hover:text-cyan-300 transition-colors">{run.fileName}</p>
                    <p className="text-xs text-slate-500">{run.passed} passed, {run.failed} failed • {run.totalDuration}ms</p>
                  </div>
                </div>
                <ArrowRight size={16} className="text-slate-600 group-hover:text-cyan-400" />
              </div>
            ))}
          </div>

        </div>
      </div>
    </>
  );

  return (
    <>
      {/* Render using portal to document.body to avoid stacking context issues */}
      {createPortal(modalContent, document.body)}

      {/* Drill down to Single Run */}
      {selectedRun && (
        <RunDetailModal 
          run={selectedRun} 
          onClose={() => setSelectedRun(null)}
          onStepSelect={onStepSelect}
        />
      )}
    </>
  );
};
