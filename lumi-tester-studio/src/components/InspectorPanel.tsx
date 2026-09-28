import React, { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { RefreshCw, ScanSearch, Square, Save } from 'lucide-react';
import { useDeviceStore, useEditorStore, useFileStore, findFile } from '../stores';

export const InspectorPanel: React.FC = () => {
  const projectRoot = useFileStore(state => state.projectRoot);
  const files = useFileStore(state => state.files);
  const dirtyFileIds = useFileStore(state => state.dirtyFileIds);
  const saveFile = useFileStore(state => state.saveFile);
  const discardFileChanges = useFileStore(state => state.discardFileChanges);
  const refreshWorkspace = useFileStore(state => state.refresh);
  const activeFileId = useEditorStore(state => state.activeFileId);
  const platform = useDeviceStore(state => state.selectedPlatform);
  const device = useDeviceStore(state => state.selectedDevice);
  const activeFile = findFile(files, activeFileId);
  const isDirty = !!activeFileId && dirtyFileIds.includes(activeFileId);
  const [port, setPort] = useState<number | null>(null);
  const [isStarting, setIsStarting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void invoke<number | null>('get_inspector_port').then(value => {
      if (!cancelled) setPort(value);
    }).catch(() => undefined);
    return () => { cancelled = true; };
  }, []);

  const start = useCallback(async () => {
    if (!projectRoot || !activeFileId || !activeFile || !/\.ya?ml$/i.test(activeFile.name)) {
      setError('Open a YAML flow first. Inspector commands are written to the active flow.');
      return;
    }
    setError(null);
    setIsStarting(true);
    try {
      if (isDirty) await saveFile(activeFileId);
      const nextPort = await invoke<number>('start_inspector', {
        workspacePath: projectRoot,
        outputPath: activeFileId,
        platform,
        device,
      });
      setPort(nextPort);
    } catch (startError) {
      setError(String(startError));
    } finally {
      setIsStarting(false);
    }
  }, [projectRoot, activeFileId, activeFile, isDirty, saveFile, platform, device]);

  const stop = async () => {
    await invoke('stop_inspector');
    setPort(null);
  };

  const applyInspectorChanges = async () => {
    if (!activeFileId) return;
    if (isDirty && !window.confirm('Reload the file from disk and discard its current unsaved editor changes?')) return;
    await discardFileChanges(activeFileId);
    await refreshWorkspace();
  };

  return (
    <section className="ide-inspector-view" aria-label="Lumi UI Inspector">
      <header className="ide-inspector-toolbar">
        <div className="ide-inspector-title"><ScanSearch size={16} /><span>UI Inspector</span><span className="ide-inspector-target">{platform}{device ? ` · ${device}` : ''}</span></div>
        <div className="ide-inspector-actions">
          {port ? (
            <>
              <button type="button" onClick={() => void applyInspectorChanges()} title="Reload commands added through Inspector"><RefreshCw size={14} /> Reload Flow</button>
              <button type="button" onClick={() => void stop()} title="Stop Inspector"><Square size={13} /> Stop</button>
            </>
          ) : (
            <button type="button" disabled={isStarting || !projectRoot} onClick={() => void start()}><ScanSearch size={14} /> {isStarting ? 'Starting…' : 'Start Inspector'}</button>
          )}
        </div>
      </header>
      {error && <div className="ide-inspector-error">{error}</div>}
      {port ? (
        <iframe className="ide-inspector-frame" title="Lumi Tester UI Inspector" src={`http://127.0.0.1:${port}/`} />
      ) : (
        <div className="ide-inspector-empty">
          <ScanSearch size={36} />
          <h2>Inspect a live screen</h2>
          <p>Connect a device or browser, open the YAML flow that should receive new commands, then start the Inspector.</p>
          {activeFile && <span><Save size={13} /> Output: {activeFile.name}</span>}
        </div>
      )}
    </section>
  );
};
