import React, { Suspense, lazy, useCallback, useEffect, useMemo, useState } from 'react';
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
import { useEditorStore, useAiStore, useFileStore, useExecutionStore } from './stores';
import { ActivityBar, ActivityId } from './components/ActivityBar';
import { CommandPalette, PaletteCommand } from './components/CommandPalette';
import { StatusBar } from './components/StatusBar';
import { PaneResizeHandle } from './components/PaneResizeHandle';
import { openDialog } from './utils/tauriUtils';
import { isTauri } from './utils/tauriUtils';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Box, Search, Sparkles, Terminal as TerminalIcon } from 'lucide-react';
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
  const runRequestVersion = useExecutionStore(state => state.runRequestVersion);
  const [activeActivity, setActiveActivity] = useState<ActivityId | null>('explorer');
  const [isPaletteOpen, setPaletteOpen] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [showHelp, setShowHelp] = useState(false);
  const [explorerWidth, setExplorerWidth] = useState(276);
  const [aiWidth, setAiWidth] = useState(360);
  const [terminalHeight, setTerminalHeight] = useState(280);
  const [isTerminalVisible, setTerminalVisible] = useState(false);

  useEffect(() => {
    if (runRequestVersion > 0) setTerminalVisible(true);
  }, [runRequestVersion]);

  const openProject = useCallback(async () => {
    const selected = await openDialog({ directory: true, multiple: false });
    if (!selected || typeof selected !== 'string') return;
    try {
      if (useFileStore.getState().dirtyFileIds.length > 0) await saveAllFiles();
      await loadProject(selected);
      setActiveActivity('explorer');
      setActiveView('editor');
    } catch (error) {
      window.alert(`Could not open workspace: ${String(error)}`);
    }
  }, [loadProject, saveAllFiles, setActiveView]);

  const commands = useMemo<PaletteCommand[]>(() => [
    { id: 'open-folder', label: 'File: Open Folder...', detail: 'Open a Lumi Tester workspace', run: openProject },
    { id: 'toggle-explorer', label: 'View: Toggle Explorer', run: () => setActiveActivity(value => value === 'explorer' ? null : 'explorer') },
    { id: 'workspace-search', label: 'Search: Find in Files', detail: 'Search text across the open workspace', run: () => { setActiveActivity('search'); setActiveView('editor'); } },
    { id: 'test-explorer', label: 'Lumi: Show Test Explorer', run: () => { setActiveActivity('tests'); setActiveView('editor'); } },
    { id: 'inspector', label: 'Lumi: Show UI Inspector', run: () => { setActiveActivity('inspector'); setActiveView('inspector'); } },
    { id: 'extensions', label: 'Lumi: Manage Extensions', run: () => { setActiveActivity('extensions'); setActiveView('extensions'); } },
    { id: 'show-editor', label: 'View: Show Editor', run: () => setActiveView('editor') },
    { id: 'show-reports', label: 'Lumi: Show Test Reports', run: () => setActiveView('report') },
    { id: 'toggle-ai', label: 'Lumi: Toggle AI Assistant', run: toggleAi },
    { id: 'toggle-terminal', label: 'View: Toggle Terminal', detail: 'Show or hide the panel', run: () => setTerminalVisible(value => !value) },
    { id: 'settings', label: 'Preferences: Open Settings', run: () => setShowSettings(true) },
  ], [openProject, setActiveView, toggleAi]);

  const handleActivitySelect = (activity: ActivityId | 'settings') => {
    if (activity === 'settings') {
      setShowSettings(true);
      return;
    }
    if (activity === activeActivity) {
      setActiveActivity(null);
      if (activity === 'reports' || activity === 'inspector' || activity === 'extensions') setActiveView('editor');
      if (activity === 'ai' && isAiOpen) toggleAi();
      return;
    }
    setActiveActivity(activity);
    if (activity !== 'ai' && isAiOpen) toggleAi();
    if (activity === 'ai') {
      if (!isAiOpen) toggleAi();
      setActiveView('editor');
    } else if (activity === 'reports') setActiveView('report');
    else if (activity === 'inspector') setActiveView('inspector');
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
      if (useFileStore.getState().dirtyFileIds.length === 0) return;
      event.preventDefault();
      try {
        await useFileStore.getState().saveAllFiles();
        await getCurrentWindow().close();
      } catch (error) {
        window.alert(`Could not save all files: ${String(error)}`);
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

  React.useEffect(() => {
    const savedRoot = localStorage.getItem('lumi_project_root');
    if (savedRoot) {
      void loadProject(savedRoot).catch(error => console.error('Could not restore workspace', error));
    }
  }, [loadProject]);

  return (
    <HashRouter>
      <div className="ide-workbench">
        <ActivityBar
          active={activeActivity}
          onSelect={handleActivitySelect}
        />
        {activeActivity === 'explorer' && <Sidebar width={explorerWidth} />}
        {activeActivity === 'explorer' && (
          <PaneResizeHandle
            orientation="vertical"
            label="Explorer"
            onResize={delta => setExplorerWidth(width => Math.max(220, Math.min(520, width + delta)))}
          />
        )}
        {activeActivity === 'search' && <Suspense fallback={<div className="ide-loading-surface">Loading search…</div>}><WorkspaceSearchPanel /></Suspense>}
        {activeActivity === 'tests' && <Suspense fallback={<div className="ide-loading-surface">Loading tests…</div>}><TestExplorer /></Suspense>}
        {activeActivity === 'extensions' && activeView !== 'extensions' && <Suspense fallback={<div className="ide-loading-surface">Loading extensions…</div>}><ExtensionsPanel /></Suspense>}

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
            {activeView === 'report'
              ? <Suspense fallback={<div className="ide-loading-surface">Loading reports…</div>}><Reports /></Suspense>
              : activeView === 'inspector'
                ? <Suspense fallback={<div className="ide-loading-surface">Loading Inspector…</div>}><InspectorPanel /></Suspense>
                : activeView === 'extensions'
                  ? <Suspense fallback={<div className="ide-loading-surface">Loading extensions…</div>}><ExtensionsPanel /></Suspense>
                  : <Editor onOpenHelp={() => setShowHelp(true)} onOpenProject={openProject} />}
            {isAiOpen && (
              <>
                <PaneResizeHandle
                  orientation="vertical"
                  label="AI Assistant"
                  reverse
                  onResize={delta => setAiWidth(width => Math.max(320, Math.min(620, width + delta)))}
                />
                <Suspense fallback={<div className="ide-ai-loading" style={{ width: aiWidth }}>Loading Lumi AI…</div>}>
                  <AiAssistant width={aiWidth} />
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
              onShow={() => setTerminalVisible(true)}
            />
          </div>
          <StatusBar />
        </main>

        {/* Modals */}
        {showSettings && <SettingsModal onClose={() => setShowSettings(false)} />}
        {showHelp && <HelpModal onClose={() => setShowHelp(false)} />}
        <CommandPalette open={isPaletteOpen} commands={commands} onClose={() => setPaletteOpen(false)} />
      </div>
    </HashRouter>
  );
};

export default App;
