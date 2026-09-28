import React, { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { FitAddon } from '@xterm/addon-fit';
import { Terminal as XTerm } from '@xterm/xterm';
import '@xterm/xterm/css/xterm.css';

interface TerminalInfo {
  id: string;
  shell: string;
  workingDirectory: string;
}

interface TerminalOutput {
  terminalId: string;
  data: string;
}

interface TerminalExit {
  terminalId: string;
  code: number | null;
}

interface PtyTerminalTabProps {
  workspacePath: string;
  active: boolean;
  onStarted: (shell: string) => void;
  onExited: (code: number | null) => void;
}

export const PtyTerminalTab: React.FC<PtyTerminalTabProps> = ({ workspacePath, active, onStarted, onExited }) => {
  const hostRef = useRef<HTMLDivElement>(null);
  const terminalIdRef = useRef<string | null>(null);
  const terminalRef = useRef<XTerm | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const onExitedRef = useRef(onExited);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => { onExitedRef.current = onExited; }, [onExited]);

  const fitAndResize = useCallback(() => {
    const terminal = terminalRef.current;
    const fit = fitRef.current;
    const terminalId = terminalIdRef.current;
    if (!terminal || !fit || !hostRef.current || !active || hostRef.current.clientWidth === 0) return;
    fit.fit();
    if (terminalId) {
      void invoke('resize_terminal', { terminalId, cols: terminal.cols, rows: terminal.rows }).catch(() => undefined);
    }
  }, [active]);

  useEffect(() => {
    const host = hostRef.current;
    if (!host) return;
    let closed = false;
    let terminalInfo: TerminalInfo | null = null;
    let unlistenOutput: UnlistenFn | undefined;
    let unlistenExit: UnlistenFn | undefined;
    let resizeObserver: ResizeObserver | undefined;
    const terminal = new XTerm({
      cursorBlink: true,
      fontFamily: 'Menlo, Monaco, "SFMono-Regular", Consolas, monospace',
      fontSize: 12,
      theme: {
        background: '#1f1f1f',
        foreground: '#cccccc',
        cursor: '#aeafad',
        selectionBackground: '#264f78',
        black: '#1e1e1e', red: '#f44747', green: '#6a9955', yellow: '#dcdcaa',
        blue: '#569cd6', magenta: '#c586c0', cyan: '#4ec9b0', white: '#d4d4d4',
        brightBlack: '#808080', brightRed: '#f44747', brightGreen: '#b5cea8',
        brightYellow: '#f9f1a5', brightBlue: '#9cdcfe', brightMagenta: '#d7ba7d',
        brightCyan: '#9cdcfe', brightWhite: '#ffffff',
      },
      allowProposedApi: false,
      scrollback: 5000,
    });
    const fit = new FitAddon();
    terminal.loadAddon(fit);
    terminal.open(host);
    terminalRef.current = terminal;
    fitRef.current = fit;

    const input = terminal.onData(data => {
      const terminalId = terminalIdRef.current;
      if (terminalId) void invoke('write_terminal', { terminalId, data }).catch(writeError => setError(String(writeError)));
    });

    const start = async () => {
      try {
        unlistenOutput = await listen<TerminalOutput>('terminal-output', event => {
          if (event.payload.terminalId === terminalIdRef.current) terminal.write(event.payload.data);
        });
        unlistenExit = await listen<TerminalExit>('terminal-exit', event => {
          if (event.payload.terminalId !== terminalIdRef.current) return;
          terminalIdRef.current = null;
          terminal.writeln(`\r\n[Process exited${event.payload.code === null ? '' : ` with code ${event.payload.code}`}].`);
          onExitedRef.current(event.payload.code);
        });
        terminalInfo = await invoke<TerminalInfo>('start_terminal', { workspacePath });
        if (closed) {
          await invoke('stop_terminal', { terminalId: terminalInfo.id });
          return;
        }
        terminalIdRef.current = terminalInfo.id;
        onStarted(terminalInfo.shell.split(/[\\/]/).filter(Boolean).pop() || terminalInfo.shell);
        requestAnimationFrame(() => {
          if (host.clientWidth > 0) fit.fit();
          if (terminalIdRef.current) {
            void invoke('resize_terminal', { terminalId: terminalIdRef.current, cols: terminal.cols, rows: terminal.rows }).catch(() => undefined);
            terminal.focus();
          }
        });
      } catch (startError) {
        setError(String(startError));
      }
    };

    void start();
    resizeObserver = new ResizeObserver(() => {
      if (active && host.clientWidth > 0) {
        requestAnimationFrame(() => {
          fit.fit();
          if (terminalIdRef.current) {
            void invoke('resize_terminal', { terminalId: terminalIdRef.current, cols: terminal.cols, rows: terminal.rows }).catch(() => undefined);
          }
        });
      }
    });
    resizeObserver.observe(host);

    return () => {
      closed = true;
      input.dispose();
      resizeObserver?.disconnect();
      unlistenOutput?.();
      unlistenExit?.();
      const terminalId = terminalIdRef.current;
      terminalIdRef.current = null;
      if (terminalId) void invoke('stop_terminal', { terminalId }).catch(() => undefined);
      terminal.dispose();
      terminalRef.current = null;
      fitRef.current = null;
    };
  }, [workspacePath]);

  useEffect(() => {
    if (active) requestAnimationFrame(fitAndResize);
  }, [active, fitAndResize]);

  return (
    <div className="ide-pty-terminal" aria-label="Interactive terminal">
      <div ref={hostRef} className="ide-pty-terminal-host" />
      {error && <div className="ide-pty-terminal-error">{error}</div>}
    </div>
  );
};
