import React from 'react';
import { useExecutionStore, useEditorStore } from '../stores';
import {
  BarChart, Bar, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer,
  LineChart, Line
} from 'recharts';
import { ArrowLeft, Download, Trash2, Eye, FileJson, ArrowRight, Folder, File, Layers } from 'lucide-react';
import { clsx } from 'clsx';
import { TestResult, StepResult } from '../types';
import { RunDetailModal } from './RunDetailModal';
import { BatchDetailModal } from './BatchDetailModal';
import { ConfirmModal } from './ConfirmModal';
import { StepDetailModal } from './StepDetailModal';

export const Reports: React.FC = () => {
  const { results, clearResults } = useExecutionStore();
  const setActiveView = useEditorStore(state => state.setActiveView);
  const [selectedRun, setSelectedRun] = React.useState<TestResult | null>(null);
  const [selectedBatchId, setSelectedBatchId] = React.useState<string | null>(null);
  const [selectedStep, setSelectedStep] = React.useState<StepResult | null>(null);
  const [viewMode, setViewMode] = React.useState<'folders' | 'single'>('folders'); // Changed 'all' to 'single'
  const [showClearConfirm, setShowClearConfirm] = React.useState(false);

  // Filter Results based on View Mode
  const singleFileResults = React.useMemo(() => results.filter(r => !r.batchId), [results]);
  const batchResults = React.useMemo(() => results.filter(r => !!r.batchId), [results]);

  // Aggregate Data (Global Stats)
  const totalRuns = results.length;
  const totalPassed = results.reduce((acc, r) => acc + r.passed, 0);
  const totalFailed = results.reduce((acc, r) => acc + r.failed, 0);
  const totalSteps = totalPassed + totalFailed;

  const passRate = totalSteps > 0 ? ((totalPassed / totalSteps) * 100).toFixed(1) : 0;

  // Timeline Data
  const timelineData = results.slice(0, 10).reverse().map(r => ({
    name: new Date(r.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
    datetime: new Date(r.timestamp).toLocaleString(),
    duration: r.totalDuration,
    passed: r.passed,
    failed: r.failed
  }));

  // Batch Data Logic
  const batches = React.useMemo(() => {
    const map = new Map<string, TestResult[]>();
    batchResults.forEach(r => {
      if (r.batchId) {
        const list = map.get(r.batchId) || [];
        list.push(r);
        map.set(r.batchId, list);
      }
    });

    return Array.from(map.entries()).map(([batchId, runs]) => {
      const sortedRuns = runs.sort((a, b) => a.timestamp - b.timestamp);
      return {
        batchId,
        folderName: sortedRuns[0].folderName,
        timestamp: sortedRuns[0].timestamp,
        totalDuration: runs.reduce((acc, r) => acc + r.totalDuration, 0),
        totalFiles: runs.length,
        passedFiles: runs.filter(r => r.failed === 0).length,
        failedFiles: runs.filter(r => r.failed > 0).length,
        runs: sortedRuns
      };
    }).sort((a, b) => b.timestamp - a.timestamp);
  }, [batchResults]);

  const CustomTooltip = ({ active, payload }: any) => {
    if (active && payload && payload.length) {
      return (
        <div className="bg-slate-900 border border-slate-700 p-3 rounded shadow-xl text-xs z-50">
          <p className="text-slate-300 font-bold mb-1">{payload[0].payload.datetime}</p>
          {payload.map((entry: any, index: number) => (
            <p key={index} style={{ color: entry.color }}>
              {entry.name}: {entry.value}
            </p>
          ))}
        </div>
      );
    }
    return null;
  };

  const handleExport = () => {
    if (results.length === 0) return;
    const blob = new Blob([JSON.stringify(results, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `nexus_report_${new Date().toISOString().split('T')[0]}.json`;
    document.body.appendChild(a);
    a.click();
    document.body.removeChild(a);
    URL.revokeObjectURL(url);
  };

  const handleClearHistory = () => {
    setShowClearConfirm(true);
  };

  const confirmClearHistory = () => {
    clearResults();
    setShowClearConfirm(false);
  };

  return (
    <div className="absolute inset-0 z-50 bg-slate-950 flex flex-col overflow-hidden">
      <div className="flex items-center justify-between p-6 border-b border-borderGlass bg-slate-900/50 backdrop-blur">
        <div className="flex items-center gap-4">
          <button
            onClick={() => setActiveView('editor')}
            className="p-2 hover:bg-slate-800 rounded-full text-slate-400 hover:text-white transition-colors"
          >
            <ArrowLeft size={20} />
          </button>
          <h1 className="text-xl font-bold text-white tracking-tight">Test Analytics Dashboard</h1>
        </div>
        <div className="flex items-center gap-3">
          <button
            onClick={handleClearHistory}
            className="flex items-center gap-2 px-4 py-2 hover:bg-rose-950/30 text-rose-400 hover:text-rose-300 rounded-lg text-sm transition-colors border border-transparent hover:border-rose-900"
            disabled={results.length === 0}
          >
            <Trash2 size={16} /> Clear History
          </button>
          <button
            onClick={handleExport}
            disabled={results.length === 0}
            className="flex items-center gap-2 px-4 py-2 bg-slate-800 hover:bg-slate-700 text-cyan-400 rounded-lg text-sm transition-colors border border-slate-700 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <Download size={16} /> Export JSON
          </button>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-8 space-y-8">

        {/* Global Statistics (Aggregated) */}
        <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
          <div className="bg-slate-900/50 p-6 rounded-2xl border border-white/5 backdrop-blur-sm">
            <h3 className="text-slate-500 text-sm font-medium mb-1">Total Runs</h3>
            <div className="flex items-end gap-2">
              <p className="text-3xl font-bold text-white">{totalRuns}</p>
              <p className="text-xs text-slate-500 mb-1.5">({singleFileResults.length} single, {batches.length} batches)</p>
            </div>
          </div>
          <div className="bg-slate-900/50 p-6 rounded-2xl border border-white/5 backdrop-blur-sm">
            <h3 className="text-slate-500 text-sm font-medium mb-1">Pass Rate</h3>
            <p className="text-3xl font-bold text-emerald-400">{passRate}%</p>
          </div>
          <div className="bg-slate-900/50 p-6 rounded-2xl border border-white/5 backdrop-blur-sm">
            <h3 className="text-slate-500 text-sm font-medium mb-1">Avg Duration</h3>
            <p className="text-3xl font-bold text-cyan-400">
              {totalRuns > 0 ? (results.reduce((acc, r) => acc + r.totalDuration, 0) / totalRuns).toFixed(0) : 0}ms
            </p>
          </div>
          <div className="bg-slate-900/50 p-6 rounded-2xl border border-white/5 backdrop-blur-sm">
            <h3 className="text-slate-500 text-sm font-medium mb-1">Total Errors</h3>
            <p className="text-3xl font-bold text-rose-400">{totalFailed}</p>
          </div>
        </div>

        {/* Charts Row */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6 h-80">
          <div className="bg-slate-900/50 p-6 rounded-2xl border border-white/5 backdrop-blur-sm flex flex-col">
            <h3 className="text-slate-200 font-semibold mb-6">Execution Performance Trend</h3>
            <div className="flex-1 min-h-0">
              <ResponsiveContainer width="100%" height="100%">
                <LineChart data={timelineData}>
                  <CartesianGrid strokeDasharray="3 3" stroke="#334155" opacity={0.3} />
                  <XAxis dataKey="name" stroke="#64748b" fontSize={10} tickLine={false} />
                  <YAxis stroke="#64748b" fontSize={10} tickLine={false} />
                  <Tooltip content={<CustomTooltip />} cursor={{ stroke: '#ffffff', strokeWidth: 1, strokeDasharray: '4 4' }} />
                  <Line type="monotone" dataKey="duration" stroke="#22d3ee" strokeWidth={2} dot={{ fill: '#22d3ee', r: 3 }} activeDot={{ r: 6 }} />
                </LineChart>
              </ResponsiveContainer>
            </div>
          </div>

          <div className="bg-slate-900/50 p-6 rounded-2xl border border-white/5 backdrop-blur-sm flex flex-col">
            <h3 className="text-slate-200 font-semibold mb-6">Step Success Ratio</h3>
            <div className="flex-1 min-h-0">
              <ResponsiveContainer width="100%" height="100%">
                <BarChart data={timelineData}>
                  <CartesianGrid strokeDasharray="3 3" stroke="#334155" opacity={0.3} vertical={false} />
                  <XAxis dataKey="name" stroke="#64748b" fontSize={10} tickLine={false} />
                  <YAxis stroke="#64748b" fontSize={10} tickLine={false} />
                  <Tooltip content={<CustomTooltip />} cursor={{ fill: '#ffffff', opacity: 0.05 }} />
                  <Bar dataKey="passed" stackId="a" fill="#10b981" radius={[0, 0, 4, 4]} />
                  <Bar dataKey="failed" stackId="a" fill="#f43f5e" radius={[4, 4, 0, 0]} />
                </BarChart>
              </ResponsiveContainer>
            </div>
          </div>
        </div>

        {/* View Tabs */}
        <div className="flex gap-6 border-b border-white/10">
          <button
            onClick={() => setViewMode('folders')}
            className={clsx(
              "pb-2 text-sm font-semibold transition-colors border-b-2 flex items-center gap-2",
              viewMode === 'folders' ? "text-cyan-400 border-cyan-400" : "text-slate-500 border-transparent hover:text-slate-300"
            )}
          >
            <Folder size={16} />
            Folder Reports
          </button>
          <button
            onClick={() => setViewMode('single')}
            className={clsx(
              "pb-2 text-sm font-semibold transition-colors border-b-2 flex items-center gap-2",
              viewMode === 'single' ? "text-cyan-400 border-cyan-400" : "text-slate-500 border-transparent hover:text-slate-300"
            )}
          >
            <File size={16} />
            Single File Reports
          </button>
        </div>

        {/* Execution History Table */}
        <div className="bg-slate-900/50 rounded-2xl border border-white/5 backdrop-blur-sm overflow-hidden min-h-[300px]">
          {viewMode === 'single' ? (
            // SINGLE FILE RUNS TABLE
            <div className="overflow-x-auto">
              <table className="w-full text-left text-sm text-slate-400">
                <thead className="bg-slate-950/50 text-slate-200 uppercase text-xs">
                  <tr>
                    <th className="px-6 py-4 font-semibold">Run ID</th>
                    <th className="px-6 py-4 font-semibold">Test File</th>
                    <th className="px-6 py-4 font-semibold">Time</th>
                    <th className="px-6 py-4 font-semibold">Result</th>
                    <th className="px-6 py-4 font-semibold text-right">Details</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-white/5">
                  {singleFileResults.map((r) => (
                    <tr
                      key={r.id}
                      className="hover:bg-slate-800/50 transition-colors cursor-pointer group"
                      onClick={() => setSelectedRun(r)}
                    >
                      <td className="px-6 py-4 font-mono text-xs opacity-70">
                        <span className="bg-slate-800 px-2 py-1 rounded">#{r.id}</span>
                      </td>
                      <td className="px-6 py-4 font-medium text-slate-300 group-hover:text-cyan-300 transition-colors">
                        <div className="flex items-center gap-2">
                          <File size={14} className="text-slate-500" />
                          {r.fileName}
                        </div>
                      </td>
                      <td className="px-6 py-4">{new Date(r.timestamp).toLocaleString()}</td>
                      <td className="px-6 py-4">
                        <span className={clsx(
                          "px-2 py-1 rounded-full text-xs font-bold",
                          (r.failed === 0 && r.passed > 0) ? "bg-emerald-500/10 text-emerald-400" :
                            r.failed > 0 ? "bg-rose-500/10 text-rose-400" : "bg-slate-700/50 text-slate-400"
                        )}>
                          {r.failed === 0 && r.passed > 0 ? 'SUCCESS' : r.failed > 0 ? 'FAILURE' : 'NO RUN'}
                        </span>
                      </td>
                      <td className="px-6 py-4 text-right">
                        <div className="flex items-center justify-end gap-3 text-xs">
                          <span className={clsx(r.passed > 0 && "text-emerald-500")}>{r.passed} passed</span>
                          <span className="text-slate-600">/</span>
                          <span className={clsx(r.failed > 0 && "text-rose-500")}>{r.failed} failed</span>
                          <Eye size={16} className="text-cyan-500 ml-2 opacity-0 group-hover:opacity-100 transition-opacity" />
                        </div>
                      </td>
                    </tr>
                  ))}
                  {singleFileResults.length === 0 && (
                    <tr>
                      <td colSpan={5} className="px-6 py-12 text-center text-slate-600 flex flex-col items-center">
                        <FileJson size={48} className="mb-4 opacity-20" />
                        <p>No single file execution history found.</p>
                        <p className="text-sm mt-1">Run individual files to see them here.</p>
                      </td>
                    </tr>
                  )}
                </tbody>
              </table>
            </div>
          ) : (
            // FOLDER RUNS TABLE
            <div className="overflow-x-auto">
              <table className="w-full text-left text-sm text-slate-400">
                <thead className="bg-slate-950/50 text-slate-200 uppercase text-xs">
                  <tr>
                    <th className="px-6 py-4 font-semibold">Folder Name</th>
                    <th className="px-6 py-4 font-semibold">Time</th>
                    <th className="px-6 py-4 font-semibold">Files Executed</th>
                    <th className="px-6 py-4 font-semibold">Status</th>
                    <th className="px-6 py-4 font-semibold text-right">Total Duration</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-white/5">
                  {batches.length === 0 ? (
                    <tr><td colSpan={5} className="px-6 py-12 text-center text-slate-600 flex flex-col items-center">
                      <Layers size={48} className="mb-4 opacity-20" />
                      <p>No folder execution history found.</p>
                      <p className="text-sm mt-1">Run a folder to generate a batch report.</p>
                    </td></tr>
                  ) : (
                    batches.map(batch => (
                      <tr key={batch.batchId} onClick={() => setSelectedBatchId(batch.batchId)} className="hover:bg-slate-800/50 transition-colors cursor-pointer group">
                        <td className="px-6 py-4 font-medium text-slate-300">
                          <div className="flex items-center gap-2">
                            <Folder size={16} className="text-cyan-500" />
                            <span>{batch.folderName || 'Unknown Folder'}</span>
                            <span className="text-xs font-mono text-slate-600 bg-slate-900 px-1.5 rounded">{batch.batchId.slice(0, 8)}...</span>
                          </div>
                        </td>
                        <td className="px-6 py-4 text-slate-400 text-xs">{new Date(batch.timestamp).toLocaleString()}</td>
                        <td className="px-6 py-4 text-slate-300">{batch.totalFiles} files</td>
                        <td className="px-6 py-4">
                          <div className="flex gap-2 text-xs font-bold">
                            {batch.passedFiles > 0 && <span className="bg-emerald-500/10 text-emerald-400 px-2 py-1 rounded">{batch.passedFiles} Passed</span>}
                            {batch.failedFiles > 0 && <span className="bg-rose-500/10 text-rose-400 px-2 py-1 rounded">{batch.failedFiles} Failed</span>}
                          </div>
                        </td>
                        <td className="px-6 py-4 text-right font-mono text-cyan-400 group-hover:text-cyan-300">
                          {batch.totalDuration}ms <ArrowRight size={14} className="inline ml-2 opacity-0 group-hover:opacity-100 transition-opacity" />
                        </td>
                      </tr>
                    ))
                  )}
                </tbody>
              </table>
            </div>
          )}
        </div>
      </div>

      {/* Modals */}
      {selectedRun && (
        <RunDetailModal
          run={selectedRun}
          onClose={() => setSelectedRun(null)}
          onStepSelect={setSelectedStep}
        />
      )}

      {selectedBatchId && (
        <BatchDetailModal
          batchId={selectedBatchId}
          runs={batches.find(b => b.batchId === selectedBatchId)?.runs || []}
          onClose={() => setSelectedBatchId(null)}
          onStepSelect={setSelectedStep}
        />
      )}

      {/* Step Detail Modal - rendered at Reports level to avoid z-index issues */}
      {selectedStep && (
        <StepDetailModal
          step={selectedStep}
          onClose={() => setSelectedStep(null)}
        />
      )}

      {/* Confirm Modal */}
      <ConfirmModal
        isOpen={showClearConfirm}
        title="Clear Execution History?"
        message="Are you sure you want to delete all test execution history? This action cannot be undone."
        confirmLabel="Yes, Clear All"
        isDangerous={true}
        onConfirm={confirmClearHistory}
        onCancel={() => setShowClearConfirm(false)}
      />
    </div>
  );
};
