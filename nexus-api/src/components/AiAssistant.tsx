import React, { useState, useRef, useEffect } from 'react';
import { useAiStore, useFileStore, getAllDescendantFiles } from '../stores';
import { generateAiResponse } from '../services/aiService';
import { X, Send, Bot, User, Settings2, Trash2, StopCircle, FileCode, Cpu } from 'lucide-react';
import { clsx } from 'clsx';
import Markdown from 'react-markdown';

export const AiAssistant: React.FC = () => {
    const {
        isAiOpen, toggleAi, aiMessages, addAiMessage,
        aiConfig, setAiConfig, isAiLoading, setAiLoading, clearAiChat
    } = useAiStore();
    const files = useFileStore(state => state.files);

    const [input, setInput] = useState('');
    const [showSettings, setShowSettings] = useState(false);
    const [mentionQuery, setMentionQuery] = useState<string | null>(null);
    const [cursorPosition, setCursorPosition] = useState(0);

    const scrollRef = useRef<HTMLDivElement>(null);
    const inputRef = useRef<HTMLTextAreaElement>(null);

    // Auto-scroll to bottom
    useEffect(() => {
        if (scrollRef.current) {
            scrollRef.current.scrollTop = scrollRef.current.scrollHeight;
        }
    }, [aiMessages, isAiLoading]);

    // Handle Input Change & Mention Detection
    const handleInputChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
        const val = e.target.value;
        const pos = e.target.selectionStart;
        setInput(val);
        setCursorPosition(pos);

        // Detect @mention
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
        setInput('');
        setMentionQuery(null);
        addAiMessage({ role: 'user', content: userMsg });
        setAiLoading(true);

        try {
            const response = await generateAiResponse(userMsg, aiConfig, files);
            addAiMessage({ role: 'model', content: response });
        } catch (e) {
            addAiMessage({ role: 'model', content: "Sorry, I encountered an error processing your request." });
        } finally {
            setAiLoading(false);
        }
    };

    const insertMention = (filename: string) => {
        if (!inputRef.current) return;
        const val = input;
        const lastAt = val.lastIndexOf('@', cursorPosition - 1);

        const newVal = val.substring(0, lastAt) + `@${filename} ` + val.substring(cursorPosition);
        setInput(newVal);
        setMentionQuery(null);
        inputRef.current.focus();
    };

    // Filter Files for Mention
    const availableFiles = React.useMemo(() => {
        if (mentionQuery === null) return [];
        const all = files.flatMap(f => getAllDescendantFiles(f));
        return all.filter(f => f.name.toLowerCase().includes(mentionQuery.toLowerCase()));
    }, [files, mentionQuery]);

    if (!isAiOpen) return null;

    return (
        <div className="w-[450px] bg-slate-950 border-l border-borderGlass flex flex-col h-full shadow-2xl relative animate-in slide-in-from-right-10 duration-300 z-30">

            {/* Header */}
            <div className="h-12 bg-slate-900 border-b border-borderGlass flex items-center justify-between px-4 shrink-0">
                <div className="flex items-center gap-2 text-cyan-400 font-bold">
                    <Bot size={18} />
                    <span>Nexus AI</span>
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
                            <label className="block text-xs text-slate-400 mb-1">Google Gemini API Key</label>
                            <input
                                type="password"
                                className="w-full bg-slate-950 border border-slate-700 rounded px-3 py-2 text-sm text-white focus:border-cyan-500 outline-none"
                                placeholder="Enter your API Key..."
                                value={aiConfig.apiKey}
                                onChange={(e) => setAiConfig({ apiKey: e.target.value })}
                            />
                            <p className="text-[10px] text-slate-600 mt-1">Leave empty to use Mock Debug Mode.</p>
                        </div>
                        <div>
                            <label className="block text-xs text-slate-400 mb-1">Model</label>
                            <select
                                className="w-full bg-slate-950 border border-slate-700 rounded px-3 py-2 text-sm text-white focus:border-cyan-500 outline-none"
                                value={aiConfig.model}
                                onChange={(e) => setAiConfig({ model: e.target.value })}
                            >
                                <option value="gemini-2.5-flash-latest">Gemini 2.5 Flash (Recommended)</option>
                                <option value="gemini-3-flash-preview">Gemini 3.0 Flash</option>
                                <option value="gemini-3-pro-preview">Gemini 3.0 Pro (Complex Logic)</option>
                            </select>
                        </div>
                    </div>
                </div>
            )}

            {/* Chat Area */}
            <div className="flex-1 overflow-y-auto p-4 space-y-4" ref={scrollRef}>
                {aiMessages.map((msg) => (
                    <div key={msg.id} className={clsx("flex gap-3 max-w-full", msg.role === 'user' ? "flex-row-reverse" : "flex-row")}>
                        <div className={clsx("w-8 h-8 rounded-full flex items-center justify-center shrink-0 mt-1", msg.role === 'user' ? "bg-cyan-600" : "bg-purple-600")}>
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
                        </div>
                    </div>
                ))}

                {isAiLoading && (
                    <div className="flex gap-3">
                        <div className="w-8 h-8 rounded-full bg-purple-600 flex items-center justify-center shrink-0">
                            <Bot size={14} />
                        </div>
                        <div className="bg-slate-800 rounded-2xl px-4 py-3 border border-white/5 flex items-center gap-2">
                            <span className="w-2 h-2 bg-purple-400 rounded-full animate-bounce"></span>
                            <span className="w-2 h-2 bg-purple-400 rounded-full animate-bounce delay-75"></span>
                            <span className="w-2 h-2 bg-purple-400 rounded-full animate-bounce delay-150"></span>
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
                            onClick={() => insertMention(f.name)}
                            className="w-full text-left px-3 py-2 text-sm text-slate-300 hover:bg-cyan-600 hover:text-white flex items-center gap-2 transition-colors"
                        >
                            <FileCode size={14} />
                            <span className="truncate">{f.name}</span>
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
                                    insertMention(availableFiles[0].name);
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
                        {isAiLoading ? <StopCircle size={18} className="animate-pulse" /> : <Send size={18} />}
                    </button>
                </div>
                <div className="flex justify-between items-center mt-2 px-1">
                    <p className="text-[10px] text-slate-500 flex items-center gap-1">
                        <Cpu size={10} />
                        {aiConfig.apiKey ? `Using ${aiConfig.model}` : 'Debug Mode (Mock)'}
                    </p>
                    <p className="text-[10px] text-slate-600">Type <span className="text-cyan-500 font-mono">@</span> to link files</p>
                </div>
            </div>
        </div>
    );
};