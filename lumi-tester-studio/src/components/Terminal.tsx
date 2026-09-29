import React, { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { Terminal as TerminalIcon, X, ChevronDown, Plus, CircleX } from 'lucide-react';
import { clsx } from 'clsx';
import { useExecutionStore, useFileStore } from '../stores';
import { PtyTerminalTab } from './PtyTerminalTab';

interface LogEntry {
  id: string;
  message: string;
  type: 'flow' | 'command' | 'log' | 'error' | 'success';
  depth: number;
  timestamp: number;
}

interface TerminalProps {
  visible: boolean;
  height: number;
  onToggle: () => void;
  onShow: () => void;
}

export const Terminal: React.FC<TerminalProps> = ({ visible, height, onToggle, onShow }) => {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [activePanel, setActivePanel] = useState<'test-output' | 'terminal'>('test-output');
  const [terminalTabs, setTerminalTabs] = useState<Array<{ key: string; label: string; running: boolean; exitCode?: number | null }>>([]);
  const [activeTerminalKey, setActiveTerminalKey] = useState<string | null>(null);
  const [terminalError, setTerminalError] = useState<string | null>(null);
  const projectRoot = useFileStore(state => state.projectRoot);
  const lastRunRequest = useExecutionStore(state => state.lastRunRequest);
  const runRequestVersion = useExecutionStore(state => state.runRequestVersion);
  const nextTerminalNumber = useRef(1);
  const scrollRef = useRef<HTMLDivElement>(null);

  useLayoutEffect(() => {
    if (!lastRunRequest || runRequestVersion === 0) return;
    onShow();
    setActivePanel('test-output');
    setLogs(previous => previous.some(log => log.id === `run-request-${lastRunRequest.id}`)
      ? previous
      : [...previous, {
          id: `run-request-${lastRunRequest.id}`,
          message: `▶ Run requested: ${lastRunRequest.label}`,
          type: 'flow',
          depth: 0,
          timestamp: Date.now(),
        }]);
  }, [lastRunRequest, onShow, runRequestVersion]);

  useEffect(() => {
    let cancelled = false;
    let unlisten: UnlistenFn | undefined;

    const setupListener = async () => {
      try {
        const stopListening = await listen<any>('test-event', (event) => {
          const payload = event.payload;
          const timestamp = Date.now();

          switch (payload.type) {
            case 'SessionStarted':
              onShow();
              setActivePanel('test-output');
              setLogs(prev => [...prev, {
                id: `session-${timestamp}`,
                message: `▶ Test session started: ${payload.session_id || 'unknown'}`,
                type: 'flow',
                depth: 0,
                timestamp
              }]);
              break;

            case 'SessionFinished':
              setLogs(prev => [...prev, {
                id: `session-finished-${timestamp}-${prev.length}`,
                message: `■ Test session finished\n  Total flows: ${payload.summary?.total_flows || 0}\n  Total commands: ${payload.summary?.total_commands || 0}\n  ${payload.summary?.passed || 0} passed, ${payload.summary?.failed || 0} failed, ${payload.summary?.skipped || 0} skipped`,
                type: 'flow',
                depth: 0,
                timestamp
              }]);
              break;

            case 'FlowStarted':
              const flowIndent = '    '.repeat(payload.depth || 0);
              setLogs(prev => [...prev, {
                id: `flow-${timestamp}-${payload.flow_name}-${prev.length}`,
                message: `${flowIndent}→ Flow: ${payload.flow_name} (${payload.command_count || 0} commands)`,
                type: 'flow',
                depth: payload.depth || 0,
                timestamp
              }]);
              break;

            case 'FlowFinished':
              const finishIndent = '    '.repeat(payload.depth || 0);
              const status = payload.status === 'Passed' ? 'PASSED' : 
                           payload.status === 'Failed' ? 'FAILED' : 
                           'UNKNOWN';
              const statusColor = payload.status === 'Passed' ? '✓' : 
                                payload.status === 'Failed' ? '✗' : '○';
              setLogs(prev => [...prev, {
                id: `flow-finished-${timestamp}-${payload.flow_name}-${prev.length}`,
                message: `${finishIndent}← Flow ${payload.flow_name} [${statusColor} ${status}]${payload.duration_ms ? ` (${payload.duration_ms}ms)` : ''}`,
                type: payload.status === 'Passed' ? 'success' : payload.status === 'Failed' ? 'error' : 'flow',
                depth: payload.depth || 0,
                timestamp
              }]);
              break;

            case 'CommandStarted':
              const cmdIndent = '    '.repeat(payload.depth || 0);
              setLogs(prev => [...prev, {
                id: `cmd-${timestamp}-${payload.index}-${prev.length}`,
                message: `${cmdIndent}[${payload.index}] ${payload.command}...`,
                type: 'command',
                depth: payload.depth || 0,
                timestamp
              }]);
              break;

            case 'CommandPassed':
              const passIndent = '    '.repeat(payload.depth || 0);
              setLogs(prev => {
                const newLogs = [...prev];
                // Update the last command log for this index
                for (let i = newLogs.length - 1; i >= 0; i--) {
                  if (newLogs[i].type === 'command' && newLogs[i].message.includes(`[${payload.index}]`)) {
                    newLogs[i] = {
                      ...newLogs[i],
                      message: `${passIndent}✓ ${newLogs[i].message.replace('...', '')} (${payload.duration_ms || 0}ms)`,
                      type: 'success'
                    };
                    break;
                  }
                }
                return newLogs;
              });
              break;

            case 'CommandFailed':
              const failIndent = '    '.repeat(payload.depth || 0);
              setLogs(prev => {
                const newLogs = [...prev];
                // Update the last command log for this index
                for (let i = newLogs.length - 1; i >= 0; i--) {
                  if (newLogs[i].type === 'command' && newLogs[i].message.includes(`[${payload.index}]`)) {
                    newLogs[i] = {
                      ...newLogs[i],
                      message: `${failIndent}✗ ${newLogs[i].message.replace('...', '')} (${payload.duration_ms || 0}ms)\n${failIndent}      Error: ${payload.error || 'Unknown error'}`,
                      type: 'error'
                    };
                    break;
                  }
                }
                return newLogs;
              });
              break;

            case 'CommandRetrying':
              const retryIndent = '    '.repeat(payload.depth || 0);
              setLogs(prev => [...prev, {
                id: `retry-${timestamp}-${payload.index}-${prev.length}`,
                message: `${retryIndent}↻ ${payload.flow_name || 'Flow'} [${payload.index}] retry ${payload.attempt || 1}/${payload.max_attempts || 1}`,
                type: 'log',
                depth: payload.depth || 0,
                timestamp,
              }]);
              break;

            case 'AppCrashed':
              const crashIndent = '    '.repeat(payload.depth || 0);
              setLogs(prev => [...prev, {
                id: `app-crashed-${timestamp}-${payload.command_index}-${prev.length}`,
                message: `${crashIndent}✗ App ${payload.app_id || 'unknown'} crashed in ${payload.flow_name || 'flow'} at command [${payload.command_index}]`,
                type: 'error',
                depth: payload.depth || 0,
                timestamp,
              }]);
              break;

            case 'CommandSkipped':
              const skipIndent = '    '.repeat(payload.depth || 0);
              setLogs(prev => {
                const newLogs = [...prev];
                for (let i = newLogs.length - 1; i >= 0; i--) {
                  if (newLogs[i].type === 'command' && newLogs[i].message.includes(`[${payload.index}]`)) {
                    newLogs[i] = {
                      ...newLogs[i],
                      message: `${skipIndent}○ ${newLogs[i].message.replace('...', '')} (${payload.reason || 'skipped'})`,
                      type: 'log'
                    };
                    break;
                  }
                }
                return newLogs;
              });
              break;

            case 'CommandAutoHealed':
              setLogs(prev => [...prev, {
                id: `auto-healed-${timestamp}-${payload.index}-${prev.length}`,
                message: `⚠ ${payload.flow_name || 'Flow'} [${payload.index}] selector auto-healed: ${payload.original_selector} → ${payload.healed_target} (${Math.round((payload.confidence || 0) * 100)}%)\n    ${payload.suggestion || ''}`,
                type: 'log',
                depth: payload.depth || 0,
                timestamp,
              }]);
              break;

            case 'Log':
              const logIndent = '    '.repeat(payload.depth || 0);
              setLogs(prev => [...prev, {
                id: `log-${timestamp}-${prev.length}`,
                message: `${logIndent}${payload.message}`,
                type: 'log',
                depth: payload.depth || 0,
                timestamp
              }]);
              break;
          }
        });

        if (cancelled) {
          stopListening();
          return;
        }
        unlisten = stopListening;
      } catch (error) {
        console.error('Failed to setup terminal listener:', error);
      }
    };

    setupListener();

    return () => {
      cancelled = true;
      if (unlisten) {
        unlisten();
      }
    };
  }, [onShow]);

  // Auto-scroll to bottom
  useEffect(() => {
    if (scrollRef.current && visible) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [logs, visible]);

  const clearLogs = () => {
    setLogs([]);
  };

  const createTerminal = () => {
    if (!projectRoot) {
      setTerminalError('Open a Lumi workspace before starting a terminal.');
      return;
    }
    setTerminalError(null);
    const key = `terminal-tab-${Date.now()}-${nextTerminalNumber.current++}`;
    setTerminalTabs(previous => [...previous, { key, label: `Terminal ${nextTerminalNumber.current - 1}`, running: true }]);
    setActiveTerminalKey(key);
    setActivePanel('terminal');
    onShow();
  };

  const closeTerminal = (key: string) => {
    const nextTabs = terminalTabs.filter(tab => tab.key !== key);
    setTerminalTabs(nextTabs);
    if (activeTerminalKey === key) setActiveTerminalKey(nextTabs.length > 0 ? nextTabs[nextTabs.length - 1].key : null);
  };

  const updateTerminalLabel = (key: string, shell: string) => {
    setTerminalTabs(previous => previous.map(tab => tab.key === key ? { ...tab, label: `${shell} ${key.split('-').pop()}` } : tab));
  };

  const markTerminalExited = (key: string, code: number | null) => {
    setTerminalTabs(previous => previous.map(tab => tab.key === key ? { ...tab, running: false, exitCode: code } : tab));
  };

  const getLogColor = (type: LogEntry['type']) => {
    switch (type) {
      case 'success':
        return 'text-emerald-400';
      case 'error':
        return 'text-rose-400';
      case 'flow':
        return 'text-cyan-400';
      case 'command':
        return 'text-slate-300';
      default:
        return 'text-slate-400';
    }
  };

  return (
    <div className={clsx(
      "ide-terminal-panel bg-slate-950 border-t border-slate-800 flex flex-col",
      !visible && "is-hidden"
    )} style={{ height: visible ? height : 0, flex: '0 0 auto' }} aria-hidden={!visible}>
      {/* Header */}
      <div className="h-8 bg-slate-900 border-b border-slate-800 flex items-center justify-between px-3 shrink-0">
        <div className="flex h-full items-center gap-1 text-[11px] font-semibold text-slate-400">
          <button
            onClick={() => { setActivePanel('test-output'); onShow(); }}
            className={clsx('h-full px-2 flex items-center gap-2 border-b-2', activePanel === 'test-output' ? 'border-cyan-400 text-slate-100' : 'border-transparent hover:text-slate-200')}
          >
            <TerminalIcon size={13} /> LUMI TEST OUTPUT
            {logs.length > 0 && <span className="text-slate-500">({logs.length})</span>}
          </button>
          <button
            onClick={() => { setActivePanel('terminal'); onShow(); }}
            className={clsx('h-full px-2 flex items-center gap-2 border-b-2', activePanel === 'terminal' ? 'border-cyan-400 text-slate-100' : 'border-transparent hover:text-slate-200')}
          >
            TERMINAL{terminalTabs.length > 0 && <span className="text-slate-500">({terminalTabs.length})</span>}
          </button>
        </div>
        <div className="flex items-center gap-1">
          {activePanel === 'test-output' ? (
            <button onClick={clearLogs} className="text-slate-500 hover:text-slate-300 p-1 rounded transition-colors" title="Clear test output">
              <X size={12} />
            </button>
          ) : (
            <>
              <button onClick={createTerminal} className="text-slate-500 hover:text-slate-300 p-1 rounded transition-colors" title="Create terminal" aria-label="Create terminal">
                <Plus size={13} />
              </button>
              {activeTerminalKey && <button onClick={() => closeTerminal(activeTerminalKey)} className="text-slate-500 hover:text-rose-400 p-1 rounded transition-colors" title="Close terminal" aria-label="Close terminal"><CircleX size={13} /></button>}
            </>
          )}
          <button
            onClick={onToggle}
            className="text-slate-500 hover:text-slate-300 p-1 rounded transition-colors"
            title="Hide Panel (⌘J / Ctrl+J)"
            aria-label="Hide Panel"
          >
            <ChevronDown size={12} />
          </button>
        </div>
      </div>

      {/* Logs */}
      <div className="flex-1 min-h-0">
        <div className={clsx('h-full overflow-y-auto p-3 font-mono text-xs', activePanel !== 'test-output' && 'hidden')} ref={scrollRef} style={{ fontFamily: 'monospace' }}>
          {logs.length === 0 ? (
            <div className="text-slate-600 text-center py-8">No logs yet. Run a test to see output here.</div>
          ) : (
            <div className="space-y-0.5">
              {logs.map((log) => <div key={log.id} className={clsx('whitespace-pre-wrap break-words', getLogColor(log.type))}>{log.message}</div>)}
            </div>
          )}
        </div>
        <div className={clsx('h-full flex flex-col', activePanel !== 'terminal' && 'hidden')}>
          <div className="h-8 shrink-0 flex items-center gap-1 px-2 border-b border-white/5 overflow-x-auto">
            {terminalTabs.map(tab => (
              <button key={tab.key} onClick={() => setActiveTerminalKey(tab.key)} className={clsx('h-6 px-2 rounded text-[11px] flex items-center gap-2 whitespace-nowrap', activeTerminalKey === tab.key ? 'bg-slate-800 text-slate-100' : 'text-slate-500 hover:text-slate-300')}>
                <span className={clsx('w-1.5 h-1.5 rounded-full', tab.running ? 'bg-emerald-400' : 'bg-slate-600')} />
                {tab.label}{!tab.running && <span className="text-slate-500">({tab.exitCode ?? 'exited'})</span>}
              </button>
            ))}
            {terminalTabs.length === 0 && <span className="text-[11px] text-slate-600">No terminal sessions</span>}
            {terminalTabs.length === 0 && <button onClick={createTerminal} className="text-[11px] text-cyan-400 hover:text-cyan-300">New Terminal</button>}
            {terminalTabs.length > 0 && <button onClick={createTerminal} className="text-slate-500 hover:text-slate-200 p-1" title="Create terminal"><Plus size={12} /></button>}
          </div>
          {terminalError && <div className="px-3 py-1 text-xs text-rose-400">{terminalError}</div>}
          <div className="flex-1 min-h-0 relative">
            {!projectRoot && terminalTabs.length === 0 && <div className="absolute inset-0 grid place-items-center text-xs text-slate-600">Open a workspace to start an interactive terminal.</div>}
            {terminalTabs.map(tab => (
              <div key={tab.key} className={clsx('absolute inset-0', activeTerminalKey !== tab.key && 'hidden')}>
                {projectRoot && <PtyTerminalTab
                  workspacePath={projectRoot}
                  active={visible && activePanel === 'terminal' && activeTerminalKey === tab.key}
                  onStarted={shell => updateTerminalLabel(tab.key, shell)}
                  onExited={code => markTerminalExited(tab.key, code)}
                />}
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
};
