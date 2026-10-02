import React, { Suspense, lazy, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Sidebar } from './components/Sidebar';
import { Editor } from './components/Editor';
import { DeviceSelector } from './components/DeviceSelector';
import { Terminal } from './components/Terminal';
import { SettingsModal } from './components/SettingsModal';
import { HelpModal } from './components/HelpModal';
const Reports = lazy(() => import('./components/Reports').then(module => ({ default: module.Reports })));
const AiAssistant = lazy(() => import('./components/AiAssistant').then(module => ({ default: module.AiAssistant })));
const TestExplorer = lazy(() => import('./components/TestExplorer').then(module => ({ default: module.TestExplorer })));
const InspectorPanel = lazy(() => import('./components/InspectorPanel').then(module => ({ default: module.InspectorPanel })));
const ExtensionsPanel = lazy(() => import('./components/ExtensionsPanel').then(module => ({ default: module.ExtensionsPanel })));
const WorkspaceSearchPanel = lazy(() => import('./components/WorkspaceSearchPanel').then(module => ({ default: module.WorkspaceSearchPanel })));
const SourceControlPanel = lazy(() => import('./components/SourceControlPanel').then(module => ({ default: module.SourceControlPanel })));
import { useEditorStore, useAiStore, useFileStore } from './stores';
import { ActivityBar, ActivityId } from './components/ActivityBar';
import { CommandPalette, PaletteCommand } from './components/CommandPalette';
import { QuickOpen } from './components/QuickOpen';
import { StatusBar } from './components/StatusBar';
import { PaneResizeHandle } from './components/PaneResizeHandle';
import { openDialog } from './utils/tauriUtils';
import { isTauri } from './utils/tauriUtils';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { Box, ScanSearch, Search, Sparkles, Terminal as TerminalIcon } from 'lucide-react';
import { HashRouter } from 'react-router-dom';
import { clsx } from 'clsx';
import './App.css';

const App: React.FC = () => {
  const activeView = useEditorStore(state => state.activeView);
  const setActiveView = useEditorStore(state => state.setActiveView);
  const { isAiOpen, toggleAi } = useAiStore();
  const loadProject = useFileStore(state => state.loadProject);
  const projectRoot = useFileStore(state => state.projectRoot);
  const saveAllFiles = useFileStore(state => state.saveAllFiles);
  const [activeActivity, setActiveActivity] = useState<ActivityId | null>('explorer');
  const [isPaletteOpen, setPaletteOpen] = useState(false);
  const [isQuickOpenOpen, setQuickOpenOpen] = useState(false);
  const [isInspectorOpen, setInspectorOpen] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [showHelp, setShowHelp] = useState(false);
  const [searchScope, setSearchScope] = useState<string | null>(null);
  const [explorerWidth, setExplorerWidth] = useState(276);
  const [aiWidth, setAiWidth] = useState(360);
  const [inspectorWidth, setInspectorWidth] = useState(420);
  const [terminalHeight, setTerminalHeight] = useState(280);
  const [isTerminalVisible, setTerminalVisible] = useState(false);
  const closeInProgress = useRef(false);
  const showTerminal = useCallback(() => setTerminalVisible(true), []);
  const searchInFolder = useCallback((relativePath: string) => {
    setSearchScope(relativePath);
    setActiveActivity('search');
    setActiveView('editor');
  }, [setActiveView]);
  const revealInExplorer = useCallback(() => {
    setActiveActivity('explorer');
    setActiveView('editor');
  }, [setActiveView]);
  const openInspector = useCallback(() => {
    setInspectorOpen(true);
    setActiveView('editor');
  }, [setActiveView]);

  const openWorkspacePath = useCallback(async (path: string) => {
    try {
      if (useFileStore.getState().dirtyFileIds.length > 0) await saveAllFiles();
      await loadProject(path);
      setActiveActivity('explorer');
      setActiveView('editor');
    } catch (error) {
      window.alert(`Could not open workspace: ${String(error)}`);
    }
  }, [loadProject, saveAllFiles, setActiveView]);

  const openProject = useCallback(async () => {
    const selected = await openDialog({ directory: true, multiple: false });
    if (!selected || typeof selected !== 'string') return;
    await openWorkspacePath(selected);
  }, [openWorkspacePath]);

  const commands = useMemo<PaletteCommand[]>(() => [
    { id: 'quick-open', label: 'File: Quick Open', detail: 'Open a file by name (⌘P / Ctrl+P)', run: () => setQuickOpenOpen(true) },
    { id: 'open-folder', label: 'File: Open Folder...', detail: 'Open a Lumi Tester workspace', run: openProject },
    { id: 'toggle-explorer', label: 'View: Toggle Explorer', run: () => setActiveActivity(value => value === 'explorer' ? null : 'explorer') },
    { id: 'workspace-search', label: 'Search: Find in Files', detail: 'Search text across the open workspace', run: () => { setActiveActivity('search'); setActiveView('editor'); } },
    { id: 'test-explorer', label: 'Lumi: Show Test Explorer', run: () => { setActiveActivity('tests'); setActiveView('editor'); } },
    { id: 'inspector', label: 'Lumi: Show UI Inspector', run: openInspector },
    { id: 'extensions', label: 'Lumi: Manage Extensions', run: () => { setActiveActivity('extensions'); setActiveView('extensions'); } },
    { id: 'show-editor', label: 'View: Show Editor', run: () => setActiveView('editor') },
    { id: 'show-reports', label: 'Lumi: Show Test Reports', run: () => setActiveView('report') },
    { id: 'toggle-ai', label: 'Lumi: Toggle AI Assistant', run: toggleAi },
    { id: 'toggle-terminal', label: 'View: Toggle Terminal', detail: 'Show or hide the panel', run: () => setTerminalVisible(value => !value) },
    { id: 'settings', label: 'Preferences: Open Settings', run: () => setShowSettings(true) },
  ], [openInspector, openProject, setActiveView, setQuickOpenOpen, toggleAi]);

  const handleActivitySelect = (activity: ActivityId | 'settings') => {
    if (activity === 'settings') {
      setShowSettings(true);
      return;
    }
    if (activity === 'inspector') {
      if (!isInspectorOpen) setActiveView('editor');
      else window.dispatchEvent(new Event('lumi-inspector-stop'));
      setInspectorOpen(value => !value);
      return;
    }
    if (activity === activeActivity) {
      setActiveActivity(null);
      if (activity === 'reports' || activity === 'extensions') setActiveView('editor');
      if (activity === 'ai' && isAiOpen) toggleAi();
      return;
    }
    setActiveActivity(activity);
    if (activity !== 'ai' && isAiOpen) toggleAi();
    if (activity === 'ai') {
      if (!isAiOpen) toggleAi();
      setActiveView('editor');
    } else if (activity === 'reports') setActiveView('report');
    else if (activity === 'extensions') setActiveView('extensions');
    else setActiveView('editor');
  };

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key.toLowerCase() === 'f') {
        event.preventDefault();
        setActiveActivity('search');
        setActiveView('editor');
        return;
      }
      if ((event.metaKey || event.ctrlKey) && !event.shiftKey && !event.altKey && event.key.toLowerCase() === 'p') {
        event.preventDefault();
        setQuickOpenOpen(true);
        return;
      }
      if ((event.metaKey || event.ctrlKey) && event.shiftKey && event.key.toLowerCase() === 'p') {
        event.preventDefault();
        setPaletteOpen(true);
        return;
      }
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'j') {
        event.preventDefault();
        setTerminalVisible(value => !value);
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);

  useEffect(() => {
    if (!isTauri()) return;
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    void getCurrentWindow().onCloseRequested(async event => {
      event.preventDefault();
      if (closeInProgress.current) {
        return;
      }
      closeInProgress.current = true;
      try {
        if (useFileStore.getState().dirtyFileIds.length > 0) {
          try {
            await useFileStore.getState().saveAllFiles();
            if (useFileStore.getState().dirtyFileIds.length > 0) {
              throw new Error('Some files changed while they were being saved. Try closing again.');
            }
          } catch (error) {
            const closeWithoutSaving = window.confirm(
              `Lumi IDE could not save your changes: ${String(error)}\n\nClose anyway and discard unsaved changes?`,
            );
            if (!closeWithoutSaving) {
              closeInProgress.current = false;
              return;
            }
          }
        }
        await invoke('exit_application');
      } catch (error) {
        closeInProgress.current = false;
        window.alert(`Could not close Lumi IDE: ${String(error)}`);
      }
    }).then(listener => {
      if (cancelled) listener();
      else unlisten = listener;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    if (!isTauri()) return;
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    void getCurrentWindow().onDragDropEvent(event => {
      if (event.payload.type !== 'drop') return;
      const path = event.payload.paths[0];
      if (path) void openWorkspacePath(path);
    }).then(listener => {
      if (cancelled) listener();
      else unlisten = listener;
    }).catch(error => console.error('Could not listen for dropped workspaces', error));
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [openWorkspacePath]);

  React.useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;
    const openPendingProjects = async () => {
      const paths = await invoke<string[]>('take_pending_recent_projects');
      for (const path of paths) {
        if (cancelled) return;
        await openWorkspacePath(path);
      }
      return paths.length;
    };
    const restoreWorkspace = async () => {
      const pendingCount = await openPendingProjects();
      if (cancelled || pendingCount || useFileStore.getState().projectRoot) return;
      const savedRoot = localStorage.getItem('lumi_project_root');
      if (savedRoot) await loadProject(savedRoot);
    };

    if (!isTauri()) {
      const savedRoot = localStorage.getItem('lumi_project_root');
      if (savedRoot) {
        void loadProject(savedRoot).catch(error => console.error('Could not restore workspace', error));
      }
      return () => { cancelled = true; };
    }

    void listen('lumi-open-recent-project', () => {
      void openPendingProjects().catch(error => console.error('Could not open recent workspace', error));
    }).then(listener => {
      if (cancelled) listener();
      else {
        unlisten = listener;
        void restoreWorkspace().catch(error => console.error('Could not restore workspace', error));
      }
    }).catch(error => console.error('Could not listen for recent workspaces', error));

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [loadProject, openWorkspacePath]);

  return (
    <HashRouter>
      <div className="ide-workbench">
        <ActivityBar
          active={activeActivity}
          inspectorOpen={isInspectorOpen}
          onSelect={handleActivitySelect}
        />
        {activeActivity === 'explorer' && <Sidebar width={explorerWidth} onSearchInFolder={searchInFolder} />}
        {activeActivity === 'explorer' && (
          <PaneResizeHandle
            orientation="vertical"
            label="Explorer"
            onResize={delta => setExplorerWidth(width => Math.max(220, Math.min(520, width + delta)))}
          />
        )}
        {activeActivity === 'search' && <Suspense fallback={<div className="ide-loading-surface">Loading search…</div>}><WorkspaceSearchPanel scopePath={searchScope} onClearScope={() => setSearchScope(null)} /></Suspense>}
        {activeActivity === 'source-control' && <Suspense fallback={<div className="ide-loading-surface">Loading Source Control…</div>}><SourceControlPanel /></Suspense>}
        {activeActivity === 'tests' && <Suspense fallback={<div className="ide-loading-surface">Loading tests…</div>}><TestExplorer /></Suspense>}
        {activeActivity === 'extensions' && activeView !== 'extensions' && <Suspense fallback={<div className="ide-loading-surface">Loading extensions…</div>}><ExtensionsPanel onOpenInspector={openInspector} /></Suspense>}

        <main className="ide-main-area">
          <header className="ide-titlebar">
            <div className="ide-brand">
              <Box size={17} />
              <span>Lumi IDE</span>
              {projectRoot && <span className="ide-workspace-name">{projectRoot.split(/[\\/]/).filter(Boolean).pop()}</span>}
            </div>
            <div className="ide-titlebar-actions">
              <DeviceSelector />
              <button
                type="button"
                onClick={openInspector}
                className={clsx('ide-ai-trigger', 'ide-inspector-trigger', isInspectorOpen && 'is-active')}
                title="Open UI Inspector"
                aria-label="Open UI Inspector"
                aria-pressed={isInspectorOpen}
              >
                <ScanSearch size={14} />
                <span>Inspect</span>
              </button>
              <button
                onClick={() => setTerminalVisible(value => !value)}
                className={clsx('ide-ai-trigger', isTerminalVisible && 'is-active')}
                title="Toggle Terminal (⌘J / Ctrl+J)"
                aria-label="Toggle Terminal"
                aria-pressed={isTerminalVisible}
              >
                <TerminalIcon size={14} />
              </button>
              <button
                onClick={toggleAi}
                className={clsx('ide-ai-trigger', isAiOpen && 'is-active')}
                title="Toggle AI Assistant"
              >
                <Sparkles size={14} />
                <span>AI</span>
              </button>
              <button className="ide-command-trigger" onClick={() => setPaletteOpen(true)} title="Search commands (⌘⇧P / Ctrl+Shift+P)" aria-label="Search commands">
                <Search size={14} />
              </button>
            </div>
          </header>

          <section className="ide-work-area">
            <div className="ide-primary-view">
              {activeView === 'report'
                ? <Suspense fallback={<div className="ide-loading-surface">Loading reports…</div>}><Reports /></Suspense>
                : activeView === 'extensions'
                  ? <Suspense fallback={<div className="ide-loading-surface">Loading extensions…</div>}><ExtensionsPanel onOpenInspector={openInspector} /></Suspense>
                  : <Editor onOpenHelp={() => setShowHelp(true)} onOpenProject={openProject} />}
            </div>
            {isInspectorOpen && (
              <>
                <PaneResizeHandle
                  orientation="vertical"
                  label="UI Inspector"
                  reverse
                  onResize={delta => setInspectorWidth(width => Math.max(320, Math.min(720, width + delta)))}
                />
                <aside className="ide-inspector-dock" style={{ width: inspectorWidth }}>
                  <Suspense fallback={<div className="ide-loading-surface">Loading Inspector…</div>}>
                    <InspectorPanel onClose={() => setInspectorOpen(false)} />
                  </Suspense>
                </aside>
              </>
            )}
            {isAiOpen && (
              <>
                <PaneResizeHandle
                  orientation="vertical"
                  label="AI Assistant"
                  reverse
                  onResize={delta => setAiWidth(width => Math.max(320, Math.min(620, width + delta)))}
                />
                <Suspense fallback={<div className="ide-ai-loading" style={{ width: aiWidth }}>Loading Lumi AI…</div>}>
                  <AiAssistant width={aiWidth} onRevealInExplorer={revealInExplorer} />
                </Suspense>
              </>
            )}
          </section>

          {isTerminalVisible && (
            <PaneResizeHandle
              orientation="horizontal"
              label="Terminal panel"
              reverse
              onResize={delta => setTerminalHeight(height => Math.max(120, Math.min(600, height + delta)))}
            />
          )}
          <div className="ide-bottom-area" style={{ height: isTerminalVisible ? terminalHeight : 0 }}>
            <Terminal
              visible={isTerminalVisible}
              height={terminalHeight}
              onToggle={() => setTerminalVisible(value => !value)}
              onShow={showTerminal}
            />
          </div>
          <StatusBar />
        </main>

        {/* Modals */}
        {showSettings && <SettingsModal onClose={() => setShowSettings(false)} />}
        {showHelp && <HelpModal onClose={() => setShowHelp(false)} />}
        <CommandPalette open={isPaletteOpen} commands={commands} onClose={() => setPaletteOpen(false)} />
        {isQuickOpenOpen && <QuickOpen onClose={() => setQuickOpenOpen(false)} />}
      </div>
    </HashRouter>
  );
};

export default App;
