import React, { useEffect, useMemo, useState } from 'react';
import yaml from 'js-yaml';
import { BadgeCheck, CircleAlert, Filter, FolderOpen, ListChecks, Play, Search, Square, XCircle } from 'lucide-react';
import { useDeviceStore, useEditorStore, useExecutionStore, useFileStore } from '../stores';
import { FileNode, TestResult } from '../types';
import { runTestFlow } from '../services/runnerService';

interface LumiTestFile {
  id: string;
  name: string;
  content: string;
  platform: string;
  tags: string[];
}

const getMetadata = (content: string) => {
  const separator = content.match(/^---\s*$/m);
  const header = separator ? content.slice(0, separator.index) : '';
  try {
    const data = (yaml.load(header) ?? {}) as { platform?: string; tags?: string[] | string };
    const tags = Array.isArray(data.tags) ? data.tags : typeof data.tags === 'string' ? [data.tags] : [];
    return { platform: data.platform ?? 'unspecified', tags };
  } catch {
    return { platform: 'unknown', tags: [] as string[] };
  }
};

const collectYamlFiles = (nodes: FileNode[]): FileNode[] => nodes.flatMap(node => {
  if (node.type === 'folder') return collectYamlFiles(node.children ?? []);
  return /\.ya?ml$/i.test(node.name) ? [node] : [];
});

const lastResultFor = (results: TestResult[], path: string) =>
  results.find(result => result.fileId === path);

export const TestExplorer: React.FC = () => {
  const projectRoot = useFileStore(state => state.projectRoot);
  const loadDescendantYamlFiles = useFileStore(state => state.loadDescendantYamlFiles);
  const openReferencedFile = useFileStore(state => state.openReferencedFile);
  const openFile = useEditorStore(state => state.openFile);
  const selectedPlatform = useDeviceStore(state => state.selectedPlatform);
  const selectedDevice = useDeviceStore(state => state.selectedDevice);
  const { queueRun, stopRun, runningNodeIds } = useExecutionStore();
  const results = useExecutionStore(state => state.results);
  const [tests, setTests] = useState<LumiTestFile[]>([]);
  const [query, setQuery] = useState('');
  const [tag, setTag] = useState('all');
  const [platform, setPlatform] = useState('all');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (!projectRoot) {
      setTests([]);
      return;
    }
    let cancelled = false;
    setIsLoading(true);
    setError(null);
    void loadDescendantYamlFiles(projectRoot).then(files => {
      if (cancelled) return;
      setTests(files.filter(file => typeof file.content === 'string').map(file => {
        const metadata = getMetadata(file.content ?? '');
        return {
          id: file.id,
          name: file.name,
          content: file.content ?? '',
          platform: metadata.platform,
          tags: metadata.tags,
        };
      }));
    }).catch(loadError => {
      if (!cancelled) setError(String(loadError));
    }).finally(() => {
      if (!cancelled) setIsLoading(false);
    });
    return () => { cancelled = true; };
  }, [projectRoot, loadDescendantYamlFiles]);

  const tags = useMemo(() => [...new Set(tests.flatMap(test => test.tags))].sort(), [tests]);
  const platforms = useMemo(() => [...new Set(tests.map(test => test.platform).filter(value => value !== 'unspecified'))].sort(), [tests]);
  const filteredTests = useMemo(() => tests.filter(test =>
    (query.trim() === '' || `${test.name} ${test.id}`.toLowerCase().includes(query.toLowerCase()))
    && (tag === 'all' || test.tags.includes(tag))
    && (platform === 'all' || test.platform === platform),
  ), [tests, query, tag, platform]);

  const openTest = async (test: LumiTestFile) => {
    const path = await openReferencedFile(test.id, test.id);
    openFile(path);
  };

  const runTests = async (selected: LumiTestFile[], suite: boolean) => {
    if (selected.length === 0) return;
    const executionStore = useExecutionStore.getState();
    const platformAtQueue = selectedPlatform;
    const deviceAtQueue = selectedDevice;
    await queueRun(suite ? `Test suite · ${selected.length} flow${selected.length === 1 ? '' : 's'}` : selected[0].name, async ({ signal, runId }) => {
      const batchId = suite ? runId : undefined;
      for (const test of selected) {
        if (signal.aborted) break;
        const runPlatform = ['android', 'ios', 'web'].includes(test.platform.toLowerCase())
          ? test.platform.toLowerCase()
          : platformAtQueue;
        executionStore.setNodeRunning(test.id, true);
        let result: TestResult;
        try {
          result = await runTestFlow(
            test.content,
            test.id,
            test.name,
            runPlatform,
            runPlatform === platformAtQueue ? deviceAtQueue : null,
            partial => {
              if (partial.id) executionStore.upsertResult({
                ...partial,
                ...(batchId ? { batchId, folderName: projectRoot?.split(/[\\/]/).filter(Boolean).pop() } : {}),
              } as TestResult);
            },
            signal,
            selected.length === 1 ? { runId } : undefined,
          );
        } finally {
          executionStore.setNodeRunning(test.id, false);
        }
        executionStore.upsertResult({
          ...result,
          ...(batchId ? { batchId, folderName: projectRoot?.split(/[\\/]/).filter(Boolean).pop() } : {}),
        });
        if (result.status === 'cancelled') break;
      }
    }).catch(runError => setError(String(runError)));
  };

  const statusIcon = (path: string) => {
    const result = lastResultFor(results, path);
    if (!result) return null;
    if (result.status === 'passed') return <BadgeCheck size={14} className="text-emerald-400" />;
    if (result.status === 'cancelled') return <CircleAlert size={14} className="text-amber-400" />;
    if (result.status === 'running') return <span className="w-2 h-2 rounded-full bg-blue-400 animate-pulse" />;
    return <XCircle size={14} className="text-rose-400" />;
  };

  return (
    <section className="ide-side-view" aria-label="Test Explorer">
      <header className="ide-side-view-header">
        <span>TEST EXPLORER</span>
        <button type="button" title="Queue all filtered tests" disabled={filteredTests.length === 0} onClick={() => void runTests(filteredTests, true)}>
          <Play size={14} />
        </button>
      </header>
      <div className="ide-test-filters">
        <label className="ide-test-search"><Search size={13} /><input value={query} onChange={event => setQuery(event.target.value)} placeholder="Filter tests" /></label>
        <label><Filter size={13} /><select value={platform} onChange={event => setPlatform(event.target.value)}><option value="all">All platforms</option>{platforms.map(value => <option key={value}>{value}</option>)}</select></label>
        <label><ListChecks size={13} /><select value={tag} onChange={event => setTag(event.target.value)}><option value="all">All tags</option>{tags.map(value => <option key={value}>{value}</option>)}</select></label>
      </div>
      <div className="ide-test-summary">
        <span>{filteredTests.length} tests</span>
        <span>{selectedPlatform} · {selectedDevice || 'select device'}</span>
      </div>
      <div className="ide-test-list">
        {isLoading && <p className="ide-view-empty">Discovering test flows…</p>}
        {!isLoading && !projectRoot && <p className="ide-view-empty">Open a workspace to discover Lumi test flows.</p>}
        {!isLoading && !!error && <p className="ide-view-error">{error}</p>}
        {!isLoading && !!projectRoot && filteredTests.length === 0 && <p className="ide-view-empty">{tests.length === 0 ? 'This workspace contains no YAML test flows.' : 'No matching YAML test flows.'}</p>}
        {filteredTests.map(test => (
          <div className="ide-test-row" key={test.id}>
            <button type="button" className="ide-test-open" onClick={() => void openTest(test)} title={test.id}>
              <span className="ide-test-status">{statusIcon(test.id)}</span>
              <span className="ide-test-name">{test.name}</span>
            </button>
            <button type="button" className="ide-test-run" onClick={() => runningNodeIds.includes(test.id) ? stopRun() : void runTests([test], false)} title={runningNodeIds.includes(test.id) ? 'Stop test' : 'Queue test'}>
              {runningNodeIds.includes(test.id) ? <Square size={13} /> : <Play size={13} />}
            </button>
            <div className="ide-test-meta"><span>{test.platform}</span>{test.tags.map(value => <span className="ide-test-tag" key={value}>{value}</span>)}</div>
          </div>
        ))}
      </div>
      {projectRoot && <footer className="ide-side-view-footer"><FolderOpen size={12} /><span>{projectRoot.split(/[\\/]/).filter(Boolean).pop()}</span></footer>}
    </section>
  );
};
