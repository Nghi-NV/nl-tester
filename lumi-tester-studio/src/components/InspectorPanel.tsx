import React, { useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { AlertCircle, Check, Copy, FileCode2, LoaderCircle, Puzzle, RefreshCw, ScanSearch, Square, X } from 'lucide-react';
import { parseAllDocuments } from 'yaml';
import { useDeviceStore, useEditorStore, useFileStore, findFile } from '../stores';

interface InspectorGuide {
  extensionName: string;
  name: string;
  description: string;
  platforms: string[];
  selectorExamples: string[];
}

interface FlowMetadata {
  platform?: string;
  appId?: string;
}

const selectablePlatforms = ['android', 'ios', 'web'] as const;
type SelectablePlatform = typeof selectablePlatforms[number];

function readFlowMetadata(content: string): FlowMetadata {
  try {
    const header = parseAllDocuments(content)[0]?.toJS();
    if (header && typeof header === 'object') {
      const value = header as Record<string, unknown>;
      return {
        platform: typeof value.platform === 'string' ? value.platform.toLowerCase() : undefined,
        appId: typeof value.appId === 'string' ? value.appId : undefined,
      };
    }
  } catch {
    // Keep Inspector usable while a flow is being edited into valid YAML.
  }

  const headerText = content.split(/^---\s*$/m, 1)[0];
  const readScalar = (key: string) => {
    const match = headerText.match(new RegExp(`^\\s*${key}\\s*:\\s*(?:"([^"]*)"|'([^']*)'|([^#\\r\\n]+))`, 'm'));
    return match?.[1] ?? match?.[2] ?? match?.[3]?.trim();
  };
  return { platform: readScalar('platform')?.toLowerCase(), appId: readScalar('appId') };
}

async function resolveInspectorDevice(platform: string): Promise<{ id?: string; name?: string }> {
  if (platform === 'web') {
    const deviceState = useDeviceStore.getState();
    if (deviceState.selectedPlatform !== 'web') deviceState.setSelectedPlatform('web');
    return {};
  }
  if (platform !== 'android' && platform !== 'ios') return {};

  const deviceStore = useDeviceStore.getState();
  if (deviceStore.selectedPlatform !== platform) deviceStore.setSelectedPlatform(platform as SelectablePlatform);
  await useDeviceStore.getState().refreshDevices();

  const refreshed = useDeviceStore.getState();
  const device = refreshed.devices.find(candidate => candidate.id === refreshed.selectedDevice);
  if (!device) {
    const label = platform === 'android' ? 'Android device or emulator' : 'iOS simulator or device';
    throw new Error(`No ${label} is available. Connect one, then retry Inspector.`);
  }
  return { id: device.id, name: device.name };
}

interface InspectorPanelProps {
  onClose?: () => void;
}

export const InspectorPanel: React.FC<InspectorPanelProps> = ({ onClose }) => {
  const projectRoot = useFileStore(state => state.projectRoot);
  const files = useFileStore(state => state.files);
  const saveFile = useFileStore(state => state.saveFile);
  const loadContent = useFileStore(state => state.loadContent);
  const discardFileChanges = useFileStore(state => state.discardFileChanges);
  const refreshWorkspace = useFileStore(state => state.refresh);
  const activeFileId = useEditorStore(state => state.activeFileId);
  const platform = useDeviceStore(state => state.selectedPlatform);
  const device = useDeviceStore(state => state.selectedDevice);
  const [outputFileId, setOutputFileId] = useState<string | null>(null);
  const inspectorFileId = outputFileId ?? activeFileId;
  const activeFile = findFile(files, inspectorFileId);
  const [port, setPort] = useState<number | null>(null);
  const [isInspectorSessionResolved, setInspectorSessionResolved] = useState(false);
  const [isStarting, setIsStarting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [inspectorGuides, setInspectorGuides] = useState<InspectorGuide[]>([]);
  const [copiedSelector, setCopiedSelector] = useState<string | null>(null);
  const [targetPlatform, setTargetPlatform] = useState<string>(platform);
  const [targetDevice, setTargetDevice] = useState<string | undefined>(device ?? undefined);
  const [appId, setAppId] = useState<string | undefined>();
  const [runningTarget, setRunningTarget] = useState<{ platform: string; device: string | null } | null>(null);
  const inspectorFrameRef = useRef<HTMLIFrameElement>(null);
  const autoStartAttemptedRef = useRef(false);

  useEffect(() => {
    let cancelled = false;
    void invoke<Array<{ manifest: { name: string; contributes: { inspectorGuides: Array<Omit<InspectorGuide, 'extensionName'>> } }; enabled: boolean }>>('list_lumi_extensions')
      .then(extensions => {
        if (cancelled) return;
        setInspectorGuides(extensions.filter(extension => extension.enabled).flatMap(extension =>
          extension.manifest.contributes.inspectorGuides.map(guide => ({
            ...guide,
            extensionName: extension.manifest.name,
          })),
        ));
      })
      .catch(() => undefined);
    return () => { cancelled = true; };
  }, []);

  const matchingGuides = inspectorGuides.filter(guide =>
    guide.platforms.length === 0 || guide.platforms.some(value => value.toLowerCase() === targetPlatform.toLowerCase()),
  );
  const isYamlFlow = activeFile?.type === 'file' && /\.ya?ml$/i.test(activeFile.name);
  const canStartInspector = Boolean(projectRoot && isYamlFlow);
  const inspectorState = port
    ? 'live'
    : isStarting
      ? 'starting'
      : !isInspectorSessionResolved
        ? 'checking'
        : error
          ? 'error'
          : !projectRoot
            ? 'workspace-required'
            : !isYamlFlow
              ? 'file-required'
              : 'ready';
  const emptyState = inspectorState === 'starting' || inspectorState === 'checking'
    ? { title: 'Connecting to Inspector…', description: 'Preparing the selected flow and target device.' }
    : inspectorState === 'error'
      ? { title: 'Inspector needs attention', description: 'Review the error above, then retry from the toolbar.' }
      : inspectorState === 'workspace-required'
        ? { title: 'Open a workspace', description: 'Choose a Lumi Tester workspace before starting UI Inspector.' }
        : inspectorState === 'file-required'
          ? { title: 'Open a YAML flow', description: 'Select a .yaml or .yml test flow in the editor to inspect its live screen.' }
          : { title: 'Ready to inspect', description: 'UI Inspector will connect to the target for the active YAML flow.' };
  const statusLabel = inspectorState === 'live'
    ? 'Live'
    : inspectorState === 'starting' || inspectorState === 'checking'
      ? 'Connecting'
      : inspectorState === 'error'
        ? 'Needs attention'
        : inspectorState === 'workspace-required'
          ? 'No workspace'
          : inspectorState === 'file-required'
            ? 'Open a flow'
            : 'Ready';

  const copySelector = async (selector: string) => {
    try {
      await navigator.clipboard.writeText(selector);
      setCopiedSelector(selector);
      window.setTimeout(() => setCopiedSelector(current => current === selector ? null : current), 1800);
    } catch (copyError) {
      setError(`Could not copy selector: ${String(copyError)}`);
    }
  };

  useEffect(() => {
    let cancelled = false;
    void Promise.all([
      invoke<number | null>('get_inspector_port'),
      invoke<string | null>('get_inspector_output_path'),
    ]).then(([value, outputPath]) => {
      if (cancelled) return;
      setPort(value);
      if (value) setOutputFileId(current => current ?? outputPath ?? activeFileId);
    }).catch(() => undefined).finally(() => {
      if (!cancelled) setInspectorSessionResolved(true);
    });
    return () => { cancelled = true; };
  }, []);

  const start = useCallback(async () => {
    if (!projectRoot || !inspectorFileId || !activeFile || !/\.ya?ml$/i.test(activeFile.name)) {
      setError('Open a YAML flow first. Inspector commands are written to the active flow.');
      return;
    }
    setError(null);
    setIsStarting(true);
    try {
      let file = findFile(useFileStore.getState().files, inspectorFileId);
      if (!file || file.type !== 'file') throw new Error('Could not read the active YAML flow.');
      if (file.content === undefined) {
        await loadContent(inspectorFileId);
        file = findFile(useFileStore.getState().files, inspectorFileId);
      }
      if (!file || file.type !== 'file' || file.content === undefined) {
        throw new Error('Could not read the active YAML flow.');
      }
      if (useFileStore.getState().dirtyFileIds.includes(inspectorFileId)) {
        await saveFile(inspectorFileId);
      }

      const metadata = readFlowMetadata(file.content);
      const target = metadata.platform || platform;
      const resolvedDevice = await resolveInspectorDevice(target);
      setTargetPlatform(target);
      setTargetDevice(resolvedDevice.name || resolvedDevice.id);
      setAppId(metadata.appId);
      const nextPort = await invoke<number>('start_inspector', {
        workspacePath: projectRoot,
        outputPath: inspectorFileId,
        platform: target,
        device: resolvedDevice.id,
      });
      setPort(nextPort);
      setOutputFileId(inspectorFileId);
      setRunningTarget({ platform: target, device: resolvedDevice.id ?? null });
    } catch (startError) {
      setError(String(startError));
    } finally {
      setIsStarting(false);
    }
  }, [projectRoot, inspectorFileId, activeFile, loadContent, saveFile, platform]);

  useEffect(() => {
    if (!isInspectorSessionResolved || port || autoStartAttemptedRef.current || !projectRoot
      || !activeFile || !/\.ya?ml$/i.test(activeFile.name)) return;
    autoStartAttemptedRef.current = true;
    void start();
  }, [isInspectorSessionResolved, port, projectRoot, activeFile?.id, start]);

  const stop = async () => {
    try {
      await invoke('stop_inspector');
      setPort(null);
      setOutputFileId(null);
      setRunningTarget(null);
    } catch (stopError) {
      setError(`Could not stop Inspector: ${String(stopError)}`);
    }
  };

  useEffect(() => {
    const handleStopRequest = () => { void stop(); };
    window.addEventListener('lumi-inspector-stop', handleStopRequest);
    return () => window.removeEventListener('lumi-inspector-stop', handleStopRequest);
  }, [port]);

  const applyInspectorChanges = async () => {
    if (!inspectorFileId) return;
    if (useFileStore.getState().dirtyFileIds.includes(inspectorFileId)
      && !window.confirm('Reload the file from disk and discard its current unsaved editor changes?')) return;
    await discardFileChanges(inspectorFileId);
    await refreshWorkspace();
  };

  useEffect(() => {
    if (!port) return;
    const handleInspectorMessage = async (event: MessageEvent) => {
      if (event.source !== inspectorFrameRef.current?.contentWindow || event.origin !== `http://127.0.0.1:${port}`) return;
      const message = event.data as { type?: unknown; value?: unknown } | null;
      if (!message || typeof message.type !== 'string') return;
      if (message.type === 'switchDevice') {
        window.dispatchEvent(new Event('lumi-open-device-selector'));
        return;
      }
      if (message.type === 'copySelector' && typeof message.value === 'string') {
        try {
          await navigator.clipboard.writeText(message.value);
        } catch (copyError) {
          setError(`Could not copy selector: ${String(copyError)}`);
        }
        return;
      }
      if (message.type !== 'insertSelector' || typeof message.value !== 'string' || !message.value.trim()) return;

      const reply = (success: boolean, error?: string) => {
        (event.source as WindowProxy | null)?.postMessage({ type: 'insertSelectorResult', success, error }, event.origin);
      };
      const destination = activeFileId;
      if (!destination) {
        const error = 'Open a YAML flow in the editor before inserting an Inspector command.';
        setError(error);
        reply(false, error);
        return;
      }

      try {
        let file = findFile(useFileStore.getState().files, destination);
        if (!file || file.type !== 'file' || !/\.ya?ml$/i.test(file.name)) {
          throw new Error('The active editor is no longer showing a YAML flow.');
        }
        if (file.content === undefined) {
          await loadContent(destination);
          file = findFile(useFileStore.getState().files, destination);
        }
        if (!file || file.type !== 'file' || file.content === undefined) {
          throw new Error('Could not read the active YAML flow.');
        }
        if (useEditorStore.getState().activeFileId !== destination) {
          throw new Error('The active YAML flow changed. Select it again, then retry the Inspector command.');
        }
        if (useEditorStore.getState().activeView !== 'editor') {
          useEditorStore.getState().setActiveView('editor');
        }
        await new Promise<void>(resolve => window.requestAnimationFrame(() => resolve()));
        if (useEditorStore.getState().activeFileId !== destination) {
          throw new Error('The active YAML flow changed. Select it again, then retry the Inspector command.');
        }

        const insertRequest = { fileId: destination, value: message.value, inserted: false };
        window.dispatchEvent(new CustomEvent('lumi-inspector-insert', { detail: insertRequest }));
        if (!insertRequest.inserted) throw new Error('Could not insert the command at the active editor cursor.');
        setError(null);
        reply(true);
      } catch (insertError) {
        const error = String(insertError);
        setError(error);
        reply(false, error);
      }
    };

    window.addEventListener('message', handleInspectorMessage);
    return () => window.removeEventListener('message', handleInspectorMessage);
  }, [port, outputFileId, activeFileId, loadContent, saveFile]);

  useEffect(() => {
    if (!port || !outputFileId || !runningTarget || !selectablePlatforms.includes(runningTarget.platform as SelectablePlatform)) return;
    if (platform === runningTarget.platform && (device ?? null) === runningTarget.device) return;

    let cancelled = false;
    setIsStarting(true);
    setPort(null);
    setError(null);
    void (async () => {
      try {
        const resolvedDevice = await resolveInspectorDevice(platform);
        const nextPort = await invoke<number>('start_inspector', {
          workspacePath: projectRoot,
          outputPath: outputFileId,
          platform,
          device: resolvedDevice.id,
        });
        if (cancelled) return;
        setTargetPlatform(platform);
        setTargetDevice(resolvedDevice.name || resolvedDevice.id);
        setPort(nextPort);
        setRunningTarget({ platform, device: resolvedDevice.id ?? null });
      } catch (restartError) {
        if (!cancelled) setError(String(restartError));
      } finally {
        if (!cancelled) setIsStarting(false);
      }
    })();

    return () => { cancelled = true; };
  }, [port, outputFileId, runningTarget, platform, device, projectRoot]);

  const inspectorUrl = (() => {
    if (!port) return '';
    const params = new URLSearchParams({ platform: targetPlatform });
    if (targetDevice) params.set('deviceName', targetDevice);
    if (appId) params.set('appId', appId);
    return `http://127.0.0.1:${port}/?${params.toString()}`;
  })();

  const close = async () => {
    if (port) await stop();
    onClose?.();
  };

  return (
    <section className="ide-inspector-view" aria-label="Lumi UI Inspector">
      <header className="ide-inspector-toolbar">
        <div className="ide-inspector-heading">
          <span className="ide-inspector-mark"><ScanSearch size={15} /></span>
          <div className="ide-inspector-heading-copy">
            <div className="ide-inspector-heading-line">
              <span className="ide-inspector-title">UI Inspector</span>
              <span className={`ide-inspector-status is-${inspectorState}`} aria-live="polite">
                <span className="ide-inspector-status-dot" />{statusLabel}
              </span>
            </div>
            <span className="ide-inspector-target" title={targetDevice ? `${targetPlatform} · ${targetDevice}` : targetPlatform}>
              {targetPlatform}{targetDevice ? ` · ${targetDevice}` : ''}
            </span>
          </div>
        </div>
        <div className="ide-inspector-actions">
          {port ? (
            <>
              <button className="ide-inspector-action" type="button" onClick={() => void applyInspectorChanges()} title="Reload the active flow from disk"><RefreshCw size={13} /><span>Reload flow</span></button>
              <button className="ide-inspector-action ide-inspector-action--icon" type="button" onClick={() => void stop()} title="Stop Inspector" aria-label="Stop Inspector"><Square size={12} /></button>
            </>
          ) : (
            <button className="ide-inspector-action ide-inspector-action--primary" type="button" disabled={isStarting || !isInspectorSessionResolved || !canStartInspector} onClick={() => void start()}>
              {isStarting ? <LoaderCircle className="ide-inspector-spinner" size={13} /> : <ScanSearch size={13} />}
              <span>{isStarting ? 'Starting…' : 'Start inspector'}</span>
            </button>
          )}
          {onClose && <button className="ide-inspector-action ide-inspector-action--icon ide-inspector-close" type="button" onClick={() => void close()} title="Close Inspector" aria-label="Close Inspector"><X size={14} /></button>}
        </div>
      </header>
      {inspectorGuides.length > 0 && (
        <details className="ide-inspector-extension-guides">
          <summary><Puzzle size={14} /><span>Extension Inspect guides</span><small>{matchingGuides.length} for {targetPlatform}</small></summary>
          {matchingGuides.length === 0
            ? <p>No installed Inspect guides match the selected platform.</p>
            : matchingGuides.map((guide, index) => (
              <article key={`${guide.extensionName}:${guide.name}:${index}`}>
                <div><strong>{guide.name}</strong><small>{guide.extensionName}</small></div>
                {guide.description && <p>{guide.description}</p>}
                {guide.selectorExamples.map((selector, selectorIndex) => (
                  <button type="button" key={`${selectorIndex}:${selector}`} title="Copy selector example" onClick={() => void copySelector(selector)}>
                    <code>{selector}</code>{copiedSelector === selector ? <Check size={12} /> : <Copy size={12} />}
                  </button>
                ))}
              </article>
            ))}
        </details>
      )}
      {error && (
        <div className="ide-inspector-error" role="alert">
          <AlertCircle size={14} />
          <span>{error}</span>
          <button type="button" onClick={() => setError(null)} title="Dismiss error" aria-label="Dismiss error"><X size={13} /></button>
        </div>
      )}
      {port ? (
        <iframe ref={inspectorFrameRef} className="ide-inspector-frame" title="Lumi Tester UI Inspector" src={inspectorUrl} />
      ) : (
        <div className={`ide-inspector-empty is-${inspectorState}`}>
          <span className="ide-inspector-empty-icon">
            {inspectorState === 'starting' || inspectorState === 'checking'
              ? <LoaderCircle className="ide-inspector-spinner" size={22} />
              : inspectorState === 'error'
                ? <AlertCircle size={22} />
                : <ScanSearch size={22} />}
          </span>
          <h2>{emptyState.title}</h2>
          <p>{emptyState.description}</p>
          {activeFile?.type === 'file' && (
            <span className="ide-inspector-flow" title={activeFile.id || activeFile.name}>
              <FileCode2 size={13} /><span>{activeFile.name}</span>
            </span>
          )}
        </div>
      )}
    </section>
  );
};
