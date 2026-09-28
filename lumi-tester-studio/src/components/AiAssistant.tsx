import React, { useState, useRef, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAiStore, useFileStore, getAllDescendantFiles } from '../stores';
import { generateAiResponse, getActiveFileContent } from '../services/aiService';
import { X, Send, Bot, User, Settings2, Trash2, LoaderCircle, FileCode, Cpu, FolderOpen, RefreshCw, Check, AlertTriangle } from 'lucide-react';
import { clsx } from 'clsx';
import Markdown from 'react-markdown';
import { openDialog } from '../utils/tauriUtils';
import { AiConfig } from '../types';

interface AiProviderInfo {
  id: 'codex' | 'agy';
  label: string;
  installed: boolean;
  binaryPath: string | null;
}

interface AiProposal {
  fileId: string;
  fileName: string;
  baseContent: string;
  proposedContent: string;
}

export const AiAssistant: React.FC<{ width?: number }> = ({ width = 360 }) => {
  const {
    isAiOpen, toggleAi, aiMessages, addAiMessage,
    aiConfig, setAiConfig, isAiLoading, setAiLoading, clearAiChat
  } = useAiStore();
  const files = useFileStore(state => state.files);
  const projectRoot = useFileStore(state => state.projectRoot);
  const updateFileContent = useFileStore(state => state.updateFileContent);

  const [input, setInput] = useState('');
  const [showSettings, setShowSettings] = useState(false);
  const [mentionQuery, setMentionQuery] = useState<string | null>(null);
  const [cursorPosition, setCursorPosition] = useState(0);
  const [detectedProviders, setDetectedProviders] = useState<AiProviderInfo[]>([]);
  const [isDetectingProviders, setIsDetectingProviders] = useState(false);
  const [proposalByMessageId, setProposalByMessageId] = useState<Record<string, AiProposal>>({});
  const [previewProposal, setPreviewProposal] = useState<AiProposal | null>(null);
  const [proposalConflict, setProposalConflict] = useState(false);

  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  // Auto-scroll to bottom
  useEffect(() => {
    if (scrollRef.current) {
      scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
    }
  }, [aiMessages, isAiLoading]);

  async function detectProviders() {
    setIsDetectingProviders(true);
    try {
      const providers = await invoke<AiProviderInfo[]>('detect_ai_providers');
      setDetectedProviders(providers);
      const selected = providers.find(provider => provider.id === aiConfig.provider);
      if (selected?.binaryPath && !aiConfig.binaryPath) setAiConfig({ binaryPath: selected.binaryPath });
    } catch {
      setDetectedProviders([]);
    } finally {
      setIsDetectingProviders(false);
    }
  }

  useEffect(() => {
    if (showSettings && aiConfig.provider !== 'gemini') void detectProviders();
  }, [showSettings]);

  // Handle Input Change & Mention Detection
  const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    const val = e.target.value;
    const pos = e.target.selectionStart;
    setInput(val);
    setCursorPosition(pos);

    // Detect @mention
    const quoteAt = val.lastIndexOf('@"', pos - 1);
    const quoteEnd = quoteAt >= 0 ? val.indexOf('"', quoteAt + 2) : -1;
    if (quoteAt >= 0 && (quoteEnd === -1 || quoteEnd >= pos)) {
      setMentionQuery(val.substring(quoteAt + 2, pos));
      return;
    }

    const lastAt = val.lastIndexOf('@', pos - 1);
    if (lastAt !== -1) {
      const textAfterAt = val.substring(lastAt + 1, pos);
      if (!textAfterAt.includes(' ')) {
        setMentionQuery(textAfterAt);
        return;
      }
    }
    setMentionQuery(null);
  };

  const handleSendMessage = async () => {
    if (!input.trim() || isAiLoading) return;

    const userMsg = input;
    const requestFiles = files;
    setInput('');
    setMentionQuery(null);
    addAiMessage({ role: 'user', content: userMsg });
    setAiLoading(true);

    try {
      const response = await generateAiResponse(userMsg, aiConfig, files, projectRoot);
      const messageId = addAiMessage({ role: 'model', content: response });
      const codeBlock = response.match(/```[^\n]*\n([\s\S]*?)```/);
      const targetMatch = response.match(/(?:^|\n)\s*(?:[-*]\s*)?(?:\*\*)?Target file(?:\*\*)?:\s*`?([^`\r\n]+)`?/i);
      const requestedPath = targetMatch?.[1]?.trim().replace(/\\/g, '/');
      const normalize = (path: string) => path.replace(/\\/g, '/').replace(/\/$/, '');
      const target = requestedPath && projectRoot
        ? requestFiles.flatMap(file => getAllDescendantFiles(file)).find(file => {
          const normalizedFile = normalize(file.id);
          const normalizedRoot = normalize(projectRoot);
          const relativePath = normalizedFile.startsWith(`${normalizedRoot}/`)
            ? normalizedFile.slice(normalizedRoot.length + 1)
            : '';
          return relativePath === requestedPath;
        })
        : undefined;
      const baseContent = target ? getActiveFileContent(requestFiles, target.id) : null;
      if (codeBlock && target && baseContent !== null) {
        setProposalByMessageId(previous => ({
          ...previous,
          [messageId]: {
            fileId: target.id,
            fileName: requestedPath ?? target.name,
            baseContent,
            proposedContent: codeBlock[1].replace(/\r?\n$/, '') + (baseContent.endsWith('\n') ? '\n' : ''),
          },
        }));
      }
    } catch (e) {
      addAiMessage({ role: 'model', content: `AI request failed: ${String(e)}` });
    } finally {
      setAiLoading(false);
    }
  };

  const openProposalPreview = (proposal: AiProposal) => {
    const currentContent = getActiveFileContent(useFileStore.getState().files, proposal.fileId);
    setProposalConflict(currentContent !== proposal.baseContent);
    setPreviewProposal(proposal);
  };

  const applyProposal = () => {
    if (!previewProposal) return;
    const currentContent = getActiveFileContent(useFileStore.getState().files, previewProposal.fileId);
    if (currentContent !== previewProposal.baseContent) {
      setProposalConflict(true);
      return;
    }
    updateFileContent(previewProposal.fileId, previewProposal.proposedContent);
    setPreviewProposal(null);
  };

  const browseProviderBinary = async () => {
    const selected = await openDialog({ directory: false, multiple: false });
    if (typeof selected === 'string') setAiConfig({ binaryPath: selected });
  };

  const insertMention = (file: { id: string; name: string }) => {
    if (!inputRef.current) return;
    const val = input;
    const lastAt = val.lastIndexOf('@', cursorPosition - 1);
    const normalizedPath = file.id.replace(/\\/g, '/');
    const normalizedRoot = (projectRoot ?? '').replace(/\\/g, '/').replace(/\/$/, '');
    const relativePath = normalizedPath.startsWith(`${normalizedRoot}/`)
      ? normalizedPath.slice(normalizedRoot.length + 1)
      : file.name;
    const mention = /\s/.test(relativePath) ? `@"${relativePath}"` : `@${relativePath}`;
    const quoteAt = val.lastIndexOf('@"', cursorPosition - 1);
    const start = quoteAt >= 0 && (val.indexOf('"', quoteAt + 2) === -1 || val.indexOf('"', quoteAt + 2) >= cursorPosition)
      ? quoteAt
      : lastAt;
    const newVal = val.substring(0, start) + `${mention} ` + val.substring(cursorPosition);
    setInput(newVal);
    setMentionQuery(null);
    inputRef.current.focus();
  };

  // Filter Files for Mention
  const availableFiles = React.useMemo(() => {
    if (mentionQuery === null) return [];
    const all = files.flatMap(f => getAllDescendantFiles(f));
    return all.filter(f => {
      const normalizedPath = f.id.replace(/\\/g, '/');
      const normalizedRoot = (projectRoot ?? '').replace(/\\/g, '/').replace(/\/$/, '');
      const relativePath = normalizedPath.startsWith(`${normalizedRoot}/`)
        ? normalizedPath.slice(normalizedRoot.length + 1)
        : f.name;
      return relativePath.toLowerCase().includes(mentionQuery.toLowerCase());
    });
  }, [files, mentionQuery, projectRoot]);
  const selectedProvider = detectedProviders.find(provider => provider.id === aiConfig.provider);

  if (!isAiOpen) return null;

  return (
    <div className="ide-ai-panel bg-slate-950 border-l border-borderGlass flex flex-col h-full shadow-2xl relative animate-in slide-in-from-right-10 duration-300 z-30" style={{ width, flex: '0 0 auto' }}>

      {/* Header */}
      <div className="ide-ai-header h-12 bg-slate-900 border-b border-borderGlass flex items-center justify-between px-4 shrink-0">
        <div className="flex items-center gap-2 text-cyan-400 font-bold">
          <Bot size={18} />
          <span>Lumi AI</span>
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={() => setShowSettings(!showSettings)}
            className={clsx("p-2 rounded hover:bg-slate-800 transition-colors", showSettings ? "text-cyan-400 bg-slate-800" : "text-slate-400")}
            title="AI Settings"
          >
            <Settings2 size={16} />
          </button>
          <button
            onClick={clearAiChat}
            className="p-2 rounded hover:bg-slate-800 text-slate-400 hover:text-rose-400 transition-colors"
            title="Clear Chat"
          >
            <Trash2 size={16} />
          </button>
          <button
            onClick={toggleAi}
            className="p-2 rounded hover:bg-slate-800 text-slate-400 hover:text-white transition-colors"
          >
            <X size={18} />
          </button>
        </div>
      </div>

      {/* Settings Panel Overlay */}
      {showSettings && (
        <div className="absolute top-12 left-0 right-0 bg-slate-900 border-b border-borderGlass p-4 z-10 shadow-xl animate-in slide-in-from-top-2">
          <h3 className="text-xs font-bold text-slate-500 uppercase mb-3">Configuration</h3>
          <div className="space-y-4">
            <div>
              <label className="block text-xs text-slate-400 mb-1">AI Provider</label>
              <select
                className="w-full bg-slate-950 border border-slate-700 rounded px-3 py-2 text-sm text-white focus:border-cyan-500 outline-none"
                value={aiConfig.provider}
                onChange={(e) => {
                  const provider = e.target.value as AiConfig['provider'];
                  const detected = detectedProviders.find(item => item.id === provider);
                  setAiConfig({ provider, binaryPath: detected?.binaryPath ?? '' });
                }}
              >
                <option value="codex">Codex CLI · local</option>
                <option value="agy">AGY · local</option>
                <option value="gemini">Gemini · API</option>
              </select>
            </div>
            {aiConfig.provider === 'gemini' ? (
              <>
                <div>
                  <label className="block text-xs text-slate-400 mb-1">Google Gemini API Key</label>
                  <input type="password" className="w-full bg-slate-950 border border-slate-700 rounded px-3 py-2 text-sm text-white focus:border-cyan-500 outline-none" placeholder="Enter your API Key..." value={aiConfig.apiKey} onChange={e => setAiConfig({ apiKey: e.target.value })} />
                  <p className="text-[10px] text-slate-600 mt-1">The API key stays in memory and is not saved with the local provider settings.</p>
                </div>
                <div>
                  <label className="block text-xs text-slate-400 mb-1">Model</label>
                  <select className="w-full bg-slate-950 border border-slate-700 rounded px-3 py-2 text-sm text-white focus:border-cyan-500 outline-none" value={aiConfig.model} onChange={e => setAiConfig({ model: e.target.value })}>
                    <option value="gemini-2.5-flash-latest">Gemini 2.5 Flash</option>
                    <option value="gemini-3-flash-preview">Gemini 3.0 Flash</option>
                    <option value="gemini-3-pro-preview">Gemini 3.0 Pro</option>
                  </select>
                </div>
              </>
            ) : (
              <div>
                <div className="flex items-center justify-between mb-1">
                  <label className="block text-xs text-slate-400">CLI binary path</label>
                  <button type="button" onClick={() => void detectProviders()} className="text-[10px] text-cyan-400 hover:text-cyan-300 flex items-center gap-1" disabled={isDetectingProviders}>
                    <RefreshCw size={11} className={isDetectingProviders ? 'animate-spin' : ''} /> Detect
                  </button>
                </div>
                <div className="flex gap-2">
                  <input className="min-w-0 flex-1 bg-slate-950 border border-slate-700 rounded px-3 py-2 text-xs text-white focus:border-cyan-500 outline-none" placeholder="Auto-detect or enter full path" value={aiConfig.binaryPath} onChange={e => setAiConfig({ binaryPath: e.target.value })} />
                  <button type="button" onClick={() => void browseProviderBinary()} className="px-2 rounded border border-slate-700 text-slate-400 hover:text-white" title="Browse for CLI"><FolderOpen size={14} /></button>
                </div>
                <p className={clsx('text-[10px] mt-1', selectedProvider?.installed || aiConfig.binaryPath.trim() ? 'text-emerald-400' : 'text-amber-400')}>
                  {selectedProvider?.installed
                    ? `Detected ${selectedProvider.label} at ${selectedProvider.binaryPath}`
                    : aiConfig.binaryPath.trim()
                      ? `Configured path: ${aiConfig.binaryPath}`
                      : 'CLI not detected. Install it or choose its executable path.'}
                </p>
                <p className="text-[10px] text-slate-600 mt-1">Codex uses read-only mode and AGY uses plan mode. Review each suggested change before staging it in the editor; local AI may inspect workspace files.</p>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Chat Area */}
      <div className="flex-1 overflow-y-auto p-4 space-y-4" ref={scrollRef}>
        {aiMessages.map((msg) => (
          <div key={msg.id} className={clsx("flex gap-3 max-w-full", msg.role === 'user' ? "flex-row-reverse" : "flex-row")}>
            <div className={clsx("w-8 h-8 rounded-full flex items-center justify-center shrink-0 mt-1", msg.role === 'user' ? "bg-cyan-600" : "bg-slate-700")}>
              {msg.role === 'user' ? <User size={14} /> : <Bot size={14} />}
            </div>
            <div className={clsx(
              "rounded-2xl px-4 py-3 text-sm max-w-[85%] leading-6 break-words shadow-md",
              msg.role === 'user' ? "bg-cyan-900/30 text-cyan-50 border border-cyan-500/20" : "bg-slate-800 text-slate-200 border border-white/5"
            )}>
              {msg.role === 'model' ? (
                <div className="prose prose-invert prose-sm max-w-none prose-pre:bg-slate-950 prose-pre:border prose-pre:border-white/10 prose-code:text-cyan-300">
                  <Markdown>{msg.content}</Markdown>
                </div>
              ) : (
                <div className="whitespace-pre-wrap">{msg.content}</div>
              )}
              {msg.role === 'model' && proposalByMessageId[msg.id] && (
                <button type="button" onClick={() => openProposalPreview(proposalByMessageId[msg.id])} className="mt-3 px-2.5 py-1.5 rounded border border-cyan-500/30 bg-cyan-500/10 text-cyan-300 text-xs hover:bg-cyan-500/20 inline-flex items-center gap-1.5">
                  <FileCode size={13} /> Review suggested file change
                </button>
              )}
            </div>
          </div>
        ))}

        {isAiLoading && (
          <div className="flex gap-3">
            <div className="w-8 h-8 rounded-full bg-slate-700 flex items-center justify-center shrink-0">
              <Bot size={14} />
            </div>
            <div className="bg-slate-800 rounded-2xl px-4 py-3 border border-white/5 flex items-center gap-2">
              <span className="w-2 h-2 bg-cyan-400 rounded-full animate-bounce"></span>
              <span className="w-2 h-2 bg-cyan-400 rounded-full animate-bounce delay-75"></span>
              <span className="w-2 h-2 bg-cyan-400 rounded-full animate-bounce delay-150"></span>
            </div>
          </div>
        )}
      </div>

      {/* Mention Popup */}
      {mentionQuery !== null && availableFiles.length > 0 && (
        <div className="absolute bottom-[70px] left-4 bg-slate-800 border border-slate-600 rounded-lg shadow-2xl max-h-48 overflow-y-auto w-64 z-50">
          <div className="px-3 py-1.5 text-[10px] text-slate-500 uppercase font-bold bg-slate-900/50">Suggested Files</div>
          {availableFiles.map(f => (
            <button
              key={f.id}
              onClick={() => insertMention(f)}
              className="w-full text-left px-3 py-2 text-sm text-slate-300 hover:bg-cyan-600 hover:text-white flex items-center gap-2 transition-colors"
            >
              <FileCode size={14} />
              <span className="truncate">{f.id.replace(/\\/g, '/').replace(`${(projectRoot ?? '').replace(/\\/g, '/').replace(/\/$/, '')}/`, '')}</span>
            </button>
          ))}
        </div>
      )}

      {/* Input Area */}
      <div className="p-4 bg-slate-900 border-t border-borderGlass shrink-0">
        <div className="relative">
          <textarea
            ref={inputRef}
            value={input}
            onChange={handleInputChange}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                if (mentionQuery !== null && availableFiles.length > 0) {
                  insertMention(availableFiles[0]);
                } else {
                  handleSendMessage();
                }
              }
              if (e.key === 'Escape') setMentionQuery(null);
            }}
            placeholder="Ask AI to write a test... (Use @ to mention files)"
            className="w-full bg-slate-950 border border-slate-700 rounded-xl pl-4 pr-12 py-3 text-sm text-white focus:border-cyan-500 outline-none resize-none h-[50px] max-h-[120px] shadow-inner"
          />
          <button
            onClick={handleSendMessage}
            disabled={!input.trim() || isAiLoading}
            className="absolute right-2 top-2 p-1.5 bg-cyan-600 hover:bg-cyan-500 text-white rounded-lg transition-colors disabled:opacity-50 disabled:cursor-not-allowed shadow-lg"
          >
            {isAiLoading ? <LoaderCircle size={18} className="animate-spin" /> : <Send size={18} />}
          </button>
        </div>
        <div className="flex justify-between items-center mt-2 px-1">
          <p className="text-[10px] text-slate-500 flex items-center gap-1">
            <Cpu size={10} />
            {aiConfig.provider === 'gemini'
              ? aiConfig.apiKey ? `Gemini · ${aiConfig.model}` : 'Gemini · API key required'
              : `${aiConfig.provider === 'codex' ? 'Codex CLI' : 'AGY'} · local`}
          </p>
          <p className="text-[10px] text-slate-600">@ to add file context</p>
        </div>
      </div>
      {previewProposal && (
        <div className="fixed inset-0 z-[100] bg-black/70 backdrop-blur-sm flex items-center justify-center p-6" role="dialog" aria-modal="true" aria-label="Review AI suggestion">
          <section className="w-full max-w-6xl max-h-[85vh] bg-slate-950 border border-slate-700 rounded-xl shadow-2xl flex flex-col overflow-hidden">
            <header className="px-5 py-4 border-b border-white/10 flex items-center justify-between">
              <div>
                <h2 className="text-sm font-semibold text-slate-100">Review AI suggestion</h2>
                <p className="text-xs text-slate-500 mt-1">{previewProposal.fileName} · changes are staged in the editor and remain unsaved</p>
              </div>
              <button type="button" onClick={() => setPreviewProposal(null)} className="text-slate-400 hover:text-white p-1" aria-label="Close suggestion preview"><X size={18} /></button>
            </header>
            {proposalConflict && <div className="px-5 py-2 bg-amber-500/10 text-amber-300 text-xs flex items-center gap-2"><AlertTriangle size={14} /> This file changed after the AI received it. Re-run the request before applying this suggestion.</div>}
            <div className="grid grid-cols-2 flex-1 min-h-0 divide-x divide-white/10">
              <div className="min-w-0 flex flex-col">
                <div className="px-4 py-2 text-[11px] font-semibold text-rose-300 border-b border-white/5">CURRENT</div>
                <pre className="flex-1 overflow-auto p-4 text-[11px] leading-5 text-slate-300 whitespace-pre-wrap font-mono">{previewProposal.baseContent}</pre>
              </div>
              <div className="min-w-0 flex flex-col">
                <div className="px-4 py-2 text-[11px] font-semibold text-emerald-300 border-b border-white/5">PROPOSED</div>
                <pre className="flex-1 overflow-auto p-4 text-[11px] leading-5 text-slate-200 whitespace-pre-wrap font-mono">{previewProposal.proposedContent}</pre>
              </div>
            </div>
            <footer className="px-5 py-3 border-t border-white/10 flex items-center justify-between">
              <span className="text-[11px] text-slate-500">The AI process cannot write files. Applying updates only the unsaved editor buffer.</span>
              <div className="flex gap-2">
                <button type="button" onClick={() => setPreviewProposal(null)} className="px-3 py-1.5 text-xs text-slate-300 hover:bg-slate-800 rounded">Cancel</button>
                <button type="button" onClick={applyProposal} disabled={proposalConflict} className="px-3 py-1.5 text-xs bg-cyan-600 hover:bg-cyan-500 disabled:opacity-40 disabled:cursor-not-allowed text-white rounded inline-flex items-center gap-1.5"><Check size={13} /> Apply to Editor</button>
              </div>
            </footer>
          </section>
        </div>
      )}
    </div>
  );
};
