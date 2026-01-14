import React, { useEffect, useRef, useState } from 'react';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { Terminal as TerminalIcon, X, Minimize2, Maximize2 } from 'lucide-react';
import { clsx } from 'clsx';

interface LogEntry {
  id: string;
  message: string;
  type: 'flow' | 'command' | 'log' | 'error' | 'success';
  depth: number;
  timestamp: number;
}

export const Terminal: React.FC = () => {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [isMinimized, setIsMinimized] = useState(true); // Mặc định minimize
  const scrollRef = useRef<HTMLDivElement>(null);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  useEffect(() => {
    let unlisten: UnlistenFn | undefined;

    const setupListener = async () => {
      try {
        unlisten = await listen<any>('test-event', (event) => {
          const payload = event.payload;
          const timestamp = Date.now();

          switch (payload.type) {
            case 'SessionStarted':
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

        unlistenRef.current = unlisten;
      } catch (error) {
        console.error('Failed to setup terminal listener:', error);
      }
    };

    setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, []);

  // Auto-scroll to bottom
  useEffect(() => {
    if (scrollRef.current && !isMinimized) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [logs, isMinimized]);

  const clearLogs = () => {
    setLogs([]);
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
      "bg-slate-950 border-t border-slate-800 flex flex-col transition-all duration-300",
      isMinimized ? "h-8" : "h-64"
    )}>
      {/* Header */}
      <div className="h-8 bg-slate-900 border-b border-slate-800 flex items-center justify-between px-3 shrink-0">
        <div className="flex items-center gap-2 text-xs font-semibold text-slate-400">
          <TerminalIcon size={14} />
          <span>TERMINAL</span>
          {logs.length > 0 && (
            <span className="text-slate-500">({logs.length})</span>
          )}
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={clearLogs}
            className="text-slate-500 hover:text-slate-300 p-1 rounded transition-colors"
            title="Clear logs"
          >
            <X size={12} />
          </button>
          <button
            onClick={() => setIsMinimized(!isMinimized)}
            className="text-slate-500 hover:text-slate-300 p-1 rounded transition-colors"
            title={isMinimized ? "Expand" : "Minimize"}
          >
            {isMinimized ? <Maximize2 size={12} /> : <Minimize2 size={12} />}
          </button>
        </div>
      </div>

      {/* Logs */}
      {!isMinimized && (
        <div
          ref={scrollRef}
          className="flex-1 overflow-y-auto p-3 font-mono text-xs"
          style={{ fontFamily: 'monospace' }}
        >
          {logs.length === 0 ? (
            <div className="text-slate-600 text-center py-8">
              No logs yet. Run a test to see output here.
            </div>
          ) : (
            <div className="space-y-0.5">
              {logs.map((log) => (
                <div
                  key={log.id}
                  className={clsx("whitespace-pre-wrap break-words", getLogColor(log.type))}
                >
                  {log.message}
                </div>
              ))}
            </div>
          )}
        </div>
      )}
    </div>
  );
};
