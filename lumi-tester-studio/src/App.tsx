import React, { useState } from 'react';
import { Sidebar } from './components/Sidebar';
import { Editor } from './components/Editor';
import { DeviceSelector } from './components/DeviceSelector';
import { Terminal } from './components/Terminal';

import { Reports } from './components/Reports';
import { SettingsModal } from './components/SettingsModal';
import { HelpModal } from './components/HelpModal';
import { AiAssistant } from './components/AiAssistant';
import { useEditorStore, useAiStore, useFileStore, useExecutionStore } from './stores';
import { Settings, Box, Sparkles, BarChart3 } from 'lucide-react';
import { HashRouter } from 'react-router-dom';
import { clsx } from 'clsx';
import './App.css';

const App: React.FC = () => {
  const activeView = useEditorStore(state => state.activeView);
  const setActiveView = useEditorStore(state => state.setActiveView);
  const { isAiOpen, toggleAi } = useAiStore();
  const loadProject = useFileStore(state => state.loadProject);
  const results = useExecutionStore(state => state.results);
  const [showSettings, setShowSettings] = useState(false);
  const [showHelp, setShowHelp] = useState(false);

  React.useEffect(() => {
    const savedRoot = localStorage.getItem('lumi_project_root');
    if (savedRoot) {
      loadProject(savedRoot);
    }
  }, [loadProject]);

  return (
    <HashRouter>
      <div className="flex h-screen w-screen bg-slate-950 text-slate-100 overflow-hidden font-sans selection:bg-cyan-500/30">

        {/* Main Layout */}
        <Sidebar />

        <div className="flex-1 flex flex-col min-w-0 relative">
          {/* Top Bar (Contextual) */}
          <div className="h-12 bg-slate-900 border-b border-borderGlass flex items-center justify-between px-4 z-20">
            <div className="flex items-center gap-2 text-cyan-400 font-bold tracking-tight">
              <Box className="w-5 h-5" />
              <span>LUMI TESTER STUDIO</span>
            </div>

            <div className="flex items-center gap-3">
              <DeviceSelector />
              <button
                onClick={toggleAi}
                className={clsx(
                  "flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm font-medium transition-all border",
                  isAiOpen
                    ? "bg-purple-600/20 text-purple-300 border-purple-500/50 shadow-[0_0_10px_rgba(168,85,247,0.2)]"
                    : "bg-slate-800 text-slate-400 border-transparent hover:text-purple-300 hover:bg-slate-700"
                )}
              >
                <Sparkles size={14} />
                AI Assistant
              </button>

              <div className="h-4 w-px bg-slate-700 mx-1"></div>

              <button
                onClick={() => setActiveView('report')}
                className={clsx(
                  "flex items-center gap-2 px-3 py-1.5 rounded-lg text-sm font-medium transition-all border",
                  activeView === 'report'
                    ? "bg-cyan-600/20 text-cyan-300 border-cyan-500/50"
                    : "bg-slate-800 text-slate-400 border-transparent hover:text-cyan-300 hover:bg-slate-700"
                )}
                title="Test Reports"
              >
                <BarChart3 size={14} />
                Reports
                {results.length > 0 && (
                  <span className="ml-1 px-1.5 py-0.5 text-xs rounded-full bg-cyan-500/20 text-cyan-300">
                    {results.length}
                  </span>
                )}
              </button>

              <button
                onClick={() => setShowSettings(true)}
                className="p-2 hover:bg-slate-800 rounded-lg text-slate-400 hover:text-cyan-400 transition-colors"
                title="Environment Settings"
              >
                <Settings size={18} />
              </button>
            </div>
          </div>

          <div className="flex-1 flex flex-col overflow-hidden relative">
            <div className="flex-1 flex overflow-hidden">
              <Editor onOpenHelp={() => setShowHelp(true)} />

              {/* Right Side Panel: AI Assistant */}
              <AiAssistant />

              {/* View Layering */}
              {activeView === 'report' && (
                <Reports />
              )}
            </div>

            {/* Terminal */}
            <Terminal />
          </div>
        </div>

        {/* Modals */}
        {showSettings && <SettingsModal onClose={() => setShowSettings(false)} />}
        {showHelp && <HelpModal onClose={() => setShowHelp(false)} />}
      </div>
    </HashRouter>
  );
};

export default App;
