import React, { useState } from 'react';
import { Sidebar } from './components/Sidebar';
import { Editor } from './components/Editor';
import { TestRunner } from './components/TestRunner';
import { Reports } from './components/Reports';
import { SettingsModal } from './components/SettingsModal';
import { HelpModal } from './components/HelpModal';
import { AiAssistant } from './components/AiAssistant';
import { useEditorStore, useAiStore } from './stores';
import { Settings, Box, Sparkles } from 'lucide-react';
import { HashRouter } from 'react-router-dom';
import { clsx } from 'clsx';

const App: React.FC = () => {
  const activeView = useEditorStore(state => state.activeView);
  const { isAiOpen, toggleAi } = useAiStore();
  const [showSettings, setShowSettings] = useState(false);
  const [showHelp, setShowHelp] = useState(false);

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
              <span>NEXUS</span>
            </div>

            <div className="flex items-center gap-3">
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
                onClick={() => setShowSettings(true)}
                className="p-2 hover:bg-slate-800 rounded-lg text-slate-400 hover:text-cyan-400 transition-colors"
                title="Environment Settings"
              >
                <Settings size={18} />
              </button>
            </div>
          </div>

          <div className="flex-1 flex overflow-hidden relative">
            <Editor onOpenHelp={() => setShowHelp(true)} />
            <TestRunner />

            {/* Right Side Panel: AI Assistant */}
            <AiAssistant />

            {/* View Layering */}
            {activeView === 'report' && (
              <Reports />
            )}
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