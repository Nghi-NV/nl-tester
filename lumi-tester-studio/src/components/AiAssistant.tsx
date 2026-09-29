import React, { useState, useRef, useEffect } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useAiStore, useFileStore, useEditorStore, getAllDescendantFiles } from '../stores';
import { generateAiResponse, getActiveFileContent, getWorkspaceRelativePath, resolveMentionedFiles, resolveWorkspaceFileLink, type AiEditorContext } from '../services/aiService';
import { AI_CONFIG } from '../constants';
import { X, Send, Bot, User, Settings2, Trash2, FileCode, Cpu, FolderOpen, RefreshCw, Check, AlertTriangle, Square, ArrowDown } from 'lucide-react';
import { clsx } from 'clsx';
import Markdown from 'react-markdown';
import { openDialog, pathJoin } from '../utils/tauriUtils';
import { AiConfig } from '../types';
import { generateRunId } from '../utils/idGenerator';

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

interface AiMentionEntry {
  relativePath: string;
  name: string;
  type: 'file' | 'folder';
}

export const AiAssistant: React.FC<{ width?: number; onRevealInExplorer?: () => void }> = ({ width = 360, onRevealInExplorer }) => {
  const {
    isAiOpen, toggleAi, aiMessages, addAiMessage,
    aiConfig, setAiConfig, isAiLoading, setAiLoading, clearAiChat
  } = useAiStore();
  const files = useFileStore(state => state.files);
  const projectRoot = useFileStore(state => state.projectRoot);
  const workspacePaths = useFileStore(state => state.workspacePaths);
  const updateFileContent = useFileStore(state => state.updateFileContent);
  const openFile = useEditorStore(state => state.openFile);
  const activeFileId = useEditorStore(state => state.activeFileId);
  const openFileIds = useEditorStore(state => state.openFiles);
  const focusedFileRelativePath = activeFileId ? getWorkspaceRelativePath(activeFileId, projectRoot) : '';

  const [input, setInput] = useState('');
  const [showSettings, setShowSettings] = useState(false);
  const [mentionQuery, setMentionQuery] = useState<string | null>(null);
  const [cursorPosition, setCursorPosition] = useState(0);
  const [detectedProviders, setDetectedProviders] = useState<AiProviderInfo[]>([]);
  const [isDetectingProviders, setIsDetectingProviders] = useState(false);
  const [proposalByMessageId, setProposalByMessageId] = useState<Record<string, AiProposal>>({});
  const [proposalIssueByMessageId, setProposalIssueByMessageId] = useState<Record<string, string>>({});
  const [previewProposal, setPreviewProposal] = useState<AiProposal | null>(null);
  const [proposalConflict, setProposalConflict] = useState(false);
  const [streamingText, setStreamingText] = useState('');
  const [showLatestButton, setShowLatestButton] = useState(false);

  const scrollRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const activeRequestRef = useRef<AbortController | null>(null);
  const streamingTextRef = useRef('');
  const streamingFrameRef = useRef<number | null>(null);
  const pinnedToLatestRef = useRef(true);
  const workspaceFiles = React.useMemo(
    () => files.flatMap(getAllDescendantFiles).filter(file => file.type === 'file'),
    [files],
  );

  const getAssistantPathFromMarkdown = (reference: string) =>
    resolveWorkspaceFileLink(reference, projectRoot, workspaceFiles);

  const openAssistantPath = async (relativePath: string, targetType: 'file' | 'folder' = 'file') => {
    const normalizedPath = relativePath.replace(/\\/g, '/').replace(/^\.\//, '');
    const root = useFileStore.getState().projectRoot ?? projectRoot;
    if (!root) return;
    if (!normalizedPath || normalizedPath.split('/').some(part => !part || part === '.' || part === '..')) return;

    if (targetType === 'folder') {
      try {
        const folderId = await pathJoin(root, ...normalizedPath.split('/'));
        onRevealInExplorer?.();
        await useFileStore.getState().revealFileInExplorer(folderId);
      } catch (error) {
        window.alert(`Could not open workspace folder: ${String(error)}`);
      }
      return;
    }

    const currentFiles = useFileStore.getState().files.flatMap(getAllDescendantFiles).filter(file => file.type === 'file');
    const exactFile = currentFiles.find(file => getWorkspaceRelativePath(file.id, root) === normalizedPath);
    if (exactFile) {
      onRevealInExplorer?.();
      await useFileStore.getState().revealFileInExplorer(exactFile.id);
      openFile(exactFile.id);
      return;
    }
    if (!normalizedPath.includes('/')) {
      const sameName = currentFiles.filter(file => file.name === normalizedPath);
      if (sameName.length === 1) {
        onRevealInExplorer?.();
        await useFileStore.getState().revealFileInExplorer(sameName[0].id);
        openFile(sameName[0].id);
        return;
      }
      if (sameName.length > 1) {
        window.alert(`Several workspace files are named ${normalizedPath}. Use the full relative path.`);
        return;
      }
    }
    try {
      const fileId = await useFileStore.getState().openWorkspaceFile(normalizedPath);
      onRevealInExplorer?.();
      await useFileStore.getState().revealFileInExplorer(fileId);
      openFile(fileId);
    } catch (error) {
      window.alert(`Could not open workspace file: ${String(error)}`);
    }
  };

  const renderUserMessage = (content: string) => {
    const parts: React.ReactNode[] = [];
    const mentionPattern = /(^|[^\w.+-])@(?:"([^"]+)"|([^\s]+))/g;
    let match: RegExpExecArray | null;
    let renderedUntil = 0;

    while ((match = mentionPattern.exec(content)) !== null) {
      const reference = (match[2] ?? match[3]).replace(/[),.;!?]+$/, '');
      if (!reference) continue;
      const mentionStart = match.index + match[1].length;
      const mentionEnd = match[2] !== undefined
        ? mentionStart + reference.length + 3
        : mentionStart + 1 + reference.length;
      const referenceQuery = /\s/.test(reference) ? `@"${reference}"` : `@${reference}`;
      const resolution = resolveMentionedFiles(referenceQuery, files, projectRoot, workspacePaths);
      const isFile = resolution.filePaths.length === 1 && resolution.folders.length === 0
        && resolution.unresolved.length === 0 && resolution.ambiguous.length === 0;
      const isFolder = resolution.folders.length === 1 && resolution.filePaths.length === 0
        && resolution.unresolved.length === 0 && resolution.ambiguous.length === 0;
      if (!isFile && !isFolder) continue;

      const type = isFile ? 'file' : 'folder';
      const resolvedPath = isFile ? resolution.filePaths[0] : resolution.folders[0];
      parts.push(content.slice(renderedUntil, mentionStart));
      parts.push(
        <button
          key={`${mentionStart}:${resolvedPath}`}
          type="button"
          onClick={() => void openAssistantPath(resolvedPath, type)}
          title={`Open ${type}: ${resolvedPath}`}
          aria-label={`Open ${type} ${resolvedPath}`}
          className="mx-0.5 inline-flex max-w-full items-center gap-1.5 align-middle rounded border border-cyan-400/30 bg-cyan-400/10 px-1.5 py-0.5 text-xs font-medium text-cyan-200 transition-colors hover:border-cyan-300/60 hover:bg-cyan-400/20 hover:text-white"
        >
          {type === 'file' ? <FileCode size={12} className="shrink-0" /> : <FolderOpen size={12} className="shrink-0" />}
          <span className="max-w-[220px] truncate">@{resolvedPath}</span>
        </button>,
      );
      renderedUntil = mentionEnd;
    }

    parts.push(content.slice(renderedUntil));
    return parts;
  };

  const markdownComponents = {
    code: ({ children, className, node, ...props }: React.ComponentProps<'code'> & { node?: unknown }) => {
      const text = String(children ?? '').replace(/\n$/, '');
      const sourcePosition = (node as { position?: { start?: { line?: number }; end?: { line?: number } } } | undefined)?.position;
      const isInlineCode = !className && sourcePosition?.start?.line === sourcePosition?.end?.line;
      const reference = isInlineCode ? getAssistantPathFromMarkdown(text) : null;
      if (reference) {
        return (
          <button
            type="button"
            onClick={() => void openAssistantPath(reference)}
            title={`Open ${reference}`}
            className="inline rounded bg-slate-950 px-1 text-cyan-300 underline decoration-cyan-500/40 underline-offset-2 hover:text-cyan-200"
          >
            {text}
          </button>
        );
      }
      return <code className={className} {...props}>{children}</code>;
    },
    a: ({ href, children, node: _node, ...props }: React.ComponentProps<'a'> & { node?: unknown }) => {
      const reference = href ? getAssistantPathFromMarkdown(href) : null;
      if (reference) {
        return (
          <button
            type="button"
            onClick={() => void openAssistantPath(reference)}
            title={`Open ${reference}`}
            className="inline cursor-pointer text-cyan-300 underline decoration-cyan-500/40 underline-offset-2 hover:text-cyan-200"
          >
            {children}
          </button>
        );
      }
      if (href && /^(?:https?:\/\/|mailto:)/i.test(href)) {
        return <a href={href} {...props} target="_blank" rel="noreferrer">{children}</a>;
      }
      return <span>{children}</span>;
    },
  };

  // Follow active output only while the tester is already reading the latest message.
  useEffect(() => {
    const scrollPane = scrollRef.current;
    if (scrollPane && pinnedToLatestRef.current) {
      scrollPane.scrollTop = scrollPane.scrollHeight;
      setShowLatestButton(false);
    }
  }, [aiMessages, isAiLoading, streamingText]);

  useEffect(() => () => {
    activeRequestRef.current?.abort();
    if (streamingFrameRef.current !== null) cancelAnimationFrame(streamingFrameRef.current);
  }, []);

  useEffect(() => {
    const textarea = inputRef.current;
    if (!textarea) return;
    textarea.style.height = 'auto';
    const height = Math.min(textarea.scrollHeight + 2, 140);
    textarea.style.height = `${height}px`;
    textarea.style.overflowY = textarea.scrollHeight + 2 > 140 ? 'auto' : 'hidden';
  }, [input]);

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
    if (lastAt !== -1 && (lastAt === 0 || !/[\w.+-]/.test(val[lastAt - 1]))) {
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
    const editorSnapshot = useEditorStore.getState();
    const fileStoreAtSubmit = useFileStore.getState();
    const priorConversation: AiEditorContext['conversationHistory'] = useAiStore.getState().aiMessages
      .filter(message => message.id !== 'init')
      .slice(-12)
      .map(message => ({ role: message.role === 'user' ? 'user' : 'assistant', content: message.content }));
    let mentionResolution = resolveMentionedFiles(userMsg, fileStoreAtSubmit.files, projectRoot, fileStoreAtSubmit.workspacePaths);
    const unexpandedPathMentions = mentionResolution.unresolved.filter(reference => reference.includes('/'));
    if (unexpandedPathMentions.length) {
      await Promise.all(unexpandedPathMentions.map(async reference => {
        const paths = /\.[^/.]+$/.test(reference)
          ? [reference]
          : [reference, `${reference}.yaml`, `${reference}.yml`];
        for (const path of paths) {
          try {
            await useFileStore.getState().openWorkspaceFile(path);
            break;
          } catch {
            // Try the next common flow extension when the mention omits it.
          }
        }
      }));
      const refreshed = useFileStore.getState();
      mentionResolution = resolveMentionedFiles(userMsg, refreshed.files, projectRoot, refreshed.workspacePaths);
    }

    const loadedRelativePaths = new Set(useFileStore.getState().files
      .flatMap(getAllDescendantFiles)
      .filter(file => file.type === 'file')
      .map(file => getWorkspaceRelativePath(file.id, projectRoot)));
    const missingMentionFiles = mentionResolution.filePaths.filter(path => !loadedRelativePaths.has(path));
    await Promise.all(missingMentionFiles.map(async path => {
      try {
        await useFileStore.getState().openWorkspaceFile(path);
      } catch {
        // The refreshed workspace index is authoritative; an unreadable path is reported below.
      }
    }));
    const afterMentionLoad = useFileStore.getState();
    mentionResolution = resolveMentionedFiles(userMsg, afterMentionLoad.files, projectRoot, afterMentionLoad.workspacePaths);
    const hydratedRelativePaths = new Set(afterMentionLoad.files
      .flatMap(getAllDescendantFiles)
      .filter(file => file.type === 'file')
      .map(file => getWorkspaceRelativePath(file.id, projectRoot)));
    const unreadableMentions = mentionResolution.filePaths.filter(path => !hydratedRelativePaths.has(path));
    mentionResolution = {
      ...mentionResolution,
      filePaths: mentionResolution.filePaths.filter(path => !unreadableMentions.includes(path)),
      unresolved: [...mentionResolution.unresolved, ...unreadableMentions],
    };

    setInput('');
    setMentionQuery(null);
    requestAnimationFrame(() => inputRef.current?.focus());
    addAiMessage({ role: 'user', content: userMsg });

    if (mentionResolution.unresolved.length || mentionResolution.ambiguous.length) {
      const unresolved = mentionResolution.unresolved.length
        ? `I couldn't find ${mentionResolution.unresolved.map(path => `@${path}`).join(', ')} in the open workspace.`
        : '';
      const ambiguous = mentionResolution.ambiguous.map(item =>
        `@${item.reference} matches multiple files: ${item.paths.map(path => `\`${path}\``).join(', ')}. Select or mention one full path.`,
      ).join('\n');
      addAiMessage({ role: 'model', content: [unresolved, ambiguous].filter(Boolean).join('\n') });
      return;
    }

    const controller = new AbortController();
    const requestId = generateRunId();
    activeRequestRef.current = controller;
    streamingTextRef.current = '';
    setStreamingText('');
    pinnedToLatestRef.current = true;
    setShowLatestButton(false);
    setAiLoading(true);

    try {
      const fileStore = useFileStore.getState();
      const availableFiles = fileStore.files.flatMap(getAllDescendantFiles);
      const contextFileIds = [...new Set([
        ...mentionResolution.filePaths.map(path => availableFiles.find(file =>
          file.type === 'file' && getWorkspaceRelativePath(file.id, projectRoot) === path,
        )?.id).filter((id): id is string => Boolean(id)),
        ...editorSnapshot.openFiles,
        ...(editorSnapshot.activeFileId ? [editorSnapshot.activeFileId] : []),
      ])];
      await Promise.all(contextFileIds.map(fileId => {
        const file = availableFiles.find(candidate => candidate.id === fileId);
        if (file?.content === undefined && !fileStore.dirtyFileIds.includes(fileId)) {
          return fileStore.loadContent(fileId);
        }
        return Promise.resolve();
      }));
      const requestFiles = useFileStore.getState().files;
      const requestContext: AiEditorContext = {
        focusedFileId: editorSnapshot.activeFileId,
        openFileIds: editorSnapshot.openFiles,
        activeView: editorSnapshot.activeView,
        activeStepName: editorSnapshot.activeStepName,
        workspacePaths: fileStore.workspacePaths,
        mentionedFolders: mentionResolution.folders,
        conversationHistory: priorConversation,
      };
      if (controller.signal.aborted) return;
      const response = await generateAiResponse(userMsg, aiConfig, requestFiles, projectRoot, requestContext, {
        requestId,
        signal: controller.signal,
        onDelta: delta => {
          streamingTextRef.current += delta;
          if (streamingFrameRef.current === null) {
            streamingFrameRef.current = requestAnimationFrame(() => {
              streamingFrameRef.current = null;
              setStreamingText(streamingTextRef.current);
            });
          }
        },
      });
      const completeResponse = response || streamingTextRef.current;
      if (!completeResponse.trim()) return;
      const messageId = addAiMessage({ role: 'model', content: completeResponse });
      if (!controller.signal.aborted) {
        const yamlBlocks = [...completeResponse.matchAll(/```([^\r\n]*)\r?\n([\s\S]*?)```/g)]
          .filter(match => ['', 'yaml', 'yml'].includes((match[1] ?? '').trim().toLowerCase()));
        const codeBlock = yamlBlocks[0];
        const targetMatch = completeResponse.match(/(?:^|\n)\s*(?:[-*]\s*)?(?:\*\*)?Target file(?:\*\*)?:\s*`?([^`\r\n]+)`?/i);
        const requestedPath = targetMatch?.[1]?.trim().replace(/^[`"']|[`"']$/g, '').replace(/\\/g, '/');
        const normalize = (path: string) => path.replace(/\\/g, '/').replace(/\/+$/, '');
        const target = requestedPath && projectRoot
          ? requestFiles.flatMap(file => getAllDescendantFiles(file)).find(file => {
            const normalizedFile = normalize(file.id);
            const normalizedRoot = normalize(projectRoot);
            const relativePath = normalizedFile.startsWith(`${normalizedRoot}/`)
              ? normalizedFile.slice(normalizedRoot.length + 1)
              : '';
            return relativePath === requestedPath
              && contextFileIds.includes(file.id)
              && file.type === 'file'
              && /\.ya?ml$/i.test(file.name);
          })
          : undefined;
        const baseContent = target ? getActiveFileContent(requestFiles, target.id) : null;
        if (codeBlock && target && baseContent !== null) {
          const proposedContent = codeBlock[2].replace(/\r?\n$/, '') + (baseContent.endsWith('\n') ? '\n' : '');
          try {
            const validation = await invoke<{ valid: boolean; diagnostics: Array<{ message: string }> }>('validate_yaml_content', {
              path: target.id,
              content: proposedContent,
            });
            if (validation.valid) {
              setProposalByMessageId(previous => ({
                ...previous,
                [messageId]: {
                  fileId: target.id,
                  fileName: requestedPath ?? target.name,
                  baseContent,
                  proposedContent,
                },
              }));
            } else {
              setProposalIssueByMessageId(previous => ({
                ...previous,
                [messageId]: validation.diagnostics[0]?.message ?? 'The suggested flow failed Lumi YAML validation.',
              }));
            }
          } catch (error) {
            setProposalIssueByMessageId(previous => ({
              ...previous,
              [messageId]: `Could not validate the suggested flow: ${String(error)}`,
            }));
          }
        }
      }
    } catch (e) {
      if (streamingTextRef.current.trim()) {
        addAiMessage({ role: 'model', content: streamingTextRef.current });
      }
      if (!controller.signal.aborted) {
        addAiMessage({ role: 'model', content: `AI request failed: ${String(e)}` });
      }
    } finally {
      if (streamingFrameRef.current !== null) {
        cancelAnimationFrame(streamingFrameRef.current);
        streamingFrameRef.current = null;
      }
      streamingTextRef.current = '';
      setStreamingText('');
      if (activeRequestRef.current === controller) activeRequestRef.current = null;
      setAiLoading(false);
    }
  };

  const stopAiResponse = () => activeRequestRef.current?.abort();

  const openProposalPreview = (proposal: AiProposal) => {
    const currentContent = getActiveFileContent(useFileStore.getState().files, proposal.fileId);
    setProposalConflict(currentContent !== proposal.baseContent);
    setPreviewProposal(proposal);
  };

  const openProposalFile = (proposal: AiProposal) => openFile(proposal.fileId);

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

  const insertMention = (entry: AiMentionEntry) => {
    if (!inputRef.current) return;
    const val = input;
    const lastAt = val.lastIndexOf('@', cursorPosition - 1);
    const relativePath = entry.type === 'folder' ? `${entry.relativePath}/` : entry.relativePath;
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
  const availableMentions = React.useMemo(() => {
    if (mentionQuery === null) return [];
    const entries = new Map<string, AiMentionEntry>();
    const addPath = (path: string, type: AiMentionEntry['type']) => {
      const relativePath = path.replace(/\\/g, '/').replace(/^\.\//, '').replace(/\/$/, '');
      if (!relativePath) return;
      entries.set(`${type}:${relativePath}`, {
        relativePath,
        name: relativePath.split('/').pop() ?? relativePath,
        type,
      });
    };
    const allPaths = new Set(workspacePaths.map(path => path.replace(/\\/g, '/')));
    const addTreePaths = (nodes: typeof files) => nodes.forEach(node => {
      const relativePath = getWorkspaceRelativePath(node.id, projectRoot) || node.name;
      if (node.type === 'file') allPaths.add(relativePath);
      else {
        addPath(relativePath, 'folder');
        if (node.children) addTreePaths(node.children);
      }
    });
    addTreePaths(files);
    for (const path of allPaths) {
      addPath(path, 'file');
      const parts = path.split('/');
      for (let index = 1; index < parts.length; index++) addPath(parts.slice(0, index).join('/'), 'folder');
    }
    return [...entries.values()]
      .filter(entry => entry.relativePath.toLowerCase().includes(mentionQuery.toLowerCase()))
      .sort((left, right) => Number(right.type === 'folder') - Number(left.type === 'folder')
        || left.relativePath.localeCompare(right.relativePath));
  }, [files, mentionQuery, projectRoot, workspacePaths]);
  const selectedProvider = detectedProviders.find(provider => provider.id === aiConfig.provider);

  if (!isAiOpen) return null;

  return (
    <div className="ide-ai-panel bg-slate-950 border-l border-borderGlass flex flex-col h-full min-w-0 max-w-full shadow-2xl relative animate-in slide-in-from-right-10 duration-300 z-30" style={{ width, flex: '0 1 auto', minWidth: 0, maxWidth: '100%' }}>

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
        <div className="absolute top-12 left-0 right-0 max-h-[calc(100%-3rem)] overflow-y-auto bg-slate-900 border-b border-borderGlass p-4 z-10 shadow-xl animate-in slide-in-from-top-2">
          <h3 className="text-xs font-bold text-slate-500 uppercase mb-3">Configuration</h3>
          <div className="space-y-4">
            <div>
              <label className="block text-xs text-slate-400 mb-1">AI Provider</label>
              <select
                className="ide-form-control text-sm"
                value={aiConfig.provider}
                onChange={(e) => {
                  const provider = e.target.value as AiConfig['provider'];
                  const detected = detectedProviders.find(item => item.id === provider);
                  setAiConfig({
                    provider,
                    binaryPath: detected?.binaryPath ?? '',
                    model: provider === 'codex'
                      ? AI_CONFIG.CHATGPT_MODEL
                      : provider === 'gemini'
                        ? 'gemini-2.5-flash-latest'
                        : '',
                  });
                }}
              >
                <option value="codex">ChatGPT · GPT-6 Luna (Codex CLI)</option>
                <option value="agy">AGY · local</option>
                <option value="gemini">Gemini · API</option>
              </select>
            </div>
            {aiConfig.provider === 'gemini' ? (
              <>
                <div>
                  <label className="block text-xs text-slate-400 mb-1">Google Gemini API Key</label>
                  <input type="password" className="ide-form-control text-sm" placeholder="Enter your API Key..." value={aiConfig.apiKey} onChange={e => setAiConfig({ apiKey: e.target.value })} />
                  <p className="text-[10px] text-slate-600 mt-1">The API key stays in memory and is not saved with the local provider settings.</p>
                </div>
                <div>
                  <label className="block text-xs text-slate-400 mb-1">Model</label>
                  <select className="ide-form-control text-sm" value={aiConfig.model} onChange={e => setAiConfig({ model: e.target.value })}>
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
                  <input className="ide-form-control text-xs flex-1" placeholder="Auto-detect or enter full path" value={aiConfig.binaryPath} onChange={e => setAiConfig({ binaryPath: e.target.value })} />
                  <button type="button" onClick={() => void browseProviderBinary()} className="px-2 rounded border border-slate-700 text-slate-400 hover:text-white" title="Browse for CLI"><FolderOpen size={14} /></button>
                </div>
                <p className={clsx('text-[10px] mt-1', selectedProvider?.installed || aiConfig.binaryPath.trim() ? 'text-emerald-400' : 'text-amber-400')}>
                  {selectedProvider?.installed
                    ? `Detected ${selectedProvider.label} at ${selectedProvider.binaryPath}`
                    : aiConfig.binaryPath.trim()
                      ? `Configured path: ${aiConfig.binaryPath}`
                      : 'CLI not detected. Install it or choose its executable path.'}
                </p>
                <p className="text-[10px] text-slate-600 mt-1">{aiConfig.provider === 'codex' ? 'Codex uses ChatGPT GPT-6 Luna in read-only mode.' : 'AGY runs locally in plan mode.'} The assistant can inspect the workspace and open tabs; review suggestions before applying them.</p>
              </div>
            )}
          </div>
        </div>
      )}

      {/* Chat Area */}
      <div className="relative flex-1 min-h-0">
      <div
        className="absolute inset-0 overflow-y-auto p-4 space-y-4"
        ref={scrollRef}
        onScroll={event => {
          const pane = event.currentTarget;
          const atBottom = pane.scrollHeight - pane.scrollTop - pane.clientHeight < 80;
          pinnedToLatestRef.current = atBottom;
          setShowLatestButton(!atBottom);
        }}
      >
        {aiMessages.map((msg) => (
          <div key={msg.id} className={clsx("flex min-w-0 gap-3 max-w-full", msg.role === 'user' ? "flex-row-reverse" : "flex-row")}>
            <div className={clsx("w-8 h-8 rounded-full flex items-center justify-center shrink-0 mt-1", msg.role === 'user' ? "bg-cyan-600" : "bg-slate-700")}>
              {msg.role === 'user' ? <User size={14} /> : <Bot size={14} />}
            </div>
            <div className={clsx(
              "min-w-0 rounded-2xl px-4 py-3 text-sm max-w-[85%] leading-6 break-words [overflow-wrap:anywhere] shadow-md",
              msg.role === 'user' ? "bg-cyan-900/30 text-cyan-50 border border-cyan-500/20" : "bg-slate-800 text-slate-200 border border-white/5"
            )}>
              {msg.role === 'model' ? (
              <div className="prose prose-invert prose-sm max-w-none prose-pre:bg-slate-950 prose-pre:border prose-pre:border-white/10 prose-code:text-cyan-300">
                  <Markdown components={markdownComponents}>{msg.content}</Markdown>
                </div>
              ) : (
                <div className="whitespace-pre-wrap">{renderUserMessage(msg.content)}</div>
              )}
              {msg.role === 'model' && proposalByMessageId[msg.id] && (
                <div className="mt-3 flex max-w-full flex-wrap items-center gap-2">
                  <button
                    type="button"
                    onClick={() => openProposalFile(proposalByMessageId[msg.id])}
                    title={`Open ${proposalByMessageId[msg.id].fileName}`}
                    className="max-w-full rounded border border-white/10 bg-slate-900 px-2.5 py-1.5 text-xs text-slate-300 hover:border-cyan-500/40 hover:text-cyan-300 inline-flex items-center gap-1.5"
                  >
                    <FileCode size={13} className="shrink-0" />
                    <span className="truncate">{proposalByMessageId[msg.id].fileName}</span>
                  </button>
                  <button type="button" onClick={() => openProposalPreview(proposalByMessageId[msg.id])} className="rounded border border-cyan-500/30 bg-cyan-500/10 px-2.5 py-1.5 text-xs text-cyan-300 hover:bg-cyan-500/20 inline-flex items-center gap-1.5">
                    <FileCode size={13} /> Review suggested change
                  </button>
                </div>
              )}
              {msg.role === 'model' && proposalIssueByMessageId[msg.id] && (
                <div className="mt-3 flex items-start gap-2 rounded border border-amber-500/25 bg-amber-500/10 px-2.5 py-2 text-xs text-amber-200">
                  <AlertTriangle size={14} className="mt-0.5 shrink-0" />
                  <span>Suggested flow was not staged because validation failed: {proposalIssueByMessageId[msg.id]}</span>
                </div>
              )}
            </div>
          </div>
        ))}

        {isAiLoading && (
          <div className="flex gap-3">
            <div className="w-8 h-8 rounded-full bg-slate-700 flex items-center justify-center shrink-0">
              <Bot size={14} />
            </div>
            <div className="min-w-0 max-w-[85%] rounded-2xl border border-white/5 bg-slate-800 px-4 py-3 text-sm leading-6 text-slate-200 shadow-md">
              {streamingText ? (
                <div className="prose prose-invert prose-sm max-w-none prose-pre:max-w-full prose-pre:overflow-x-auto prose-pre:bg-slate-950 prose-pre:border prose-pre:border-white/10 prose-code:text-cyan-300">
                  <Markdown components={markdownComponents}>{streamingText}</Markdown>
                  <span aria-hidden="true" className="ml-1 inline-block h-4 w-[2px] animate-pulse rounded bg-cyan-300 align-text-bottom" />
                </div>
              ) : (
                <div className="flex items-center gap-2" role="status" aria-live="polite">
                  <span className="w-1.5 h-1.5 rounded-full bg-cyan-400 animate-pulse" />
                  <span className="text-xs text-slate-400">Thinking</span>
                </div>
              )}
            </div>
          </div>
        )}
      </div>
      {showLatestButton && (
        <button
          type="button"
          onClick={() => {
            pinnedToLatestRef.current = true;
            setShowLatestButton(false);
            scrollRef.current?.scrollTo({ top: scrollRef.current.scrollHeight, behavior: 'smooth' });
          }}
          className="absolute bottom-4 right-5 z-10 inline-flex items-center gap-1.5 rounded-full border border-slate-600 bg-slate-800/95 px-3 py-1.5 text-xs text-slate-200 shadow-lg transition-colors hover:border-cyan-500/50 hover:text-cyan-200"
        >
          <ArrowDown size={13} /> Latest response
        </button>
      )}
      </div>

      {/* Input Area */}
      <div className="relative shrink-0 border-t border-borderGlass bg-slate-900 p-3 sm:p-4">
        {mentionQuery !== null && (
          <div className="absolute bottom-full left-3 right-3 z-50 mb-2 max-h-48 min-w-0 overflow-y-auto rounded-lg border border-slate-600 bg-slate-800 shadow-2xl sm:left-4 sm:right-4" role="listbox" aria-label="Files and folders">
            <div className="sticky top-0 bg-slate-900/95 px-3 py-1.5 text-[10px] font-bold uppercase text-slate-500">Files and folders</div>
            {availableMentions.length > 0 ? availableMentions.map(entry => (
              <button
                key={`${entry.type}:${entry.relativePath}`}
                type="button"
                role="option"
                onClick={() => insertMention(entry)}
                className="flex w-full min-w-0 items-center gap-2 px-3 py-2 text-left text-sm text-slate-300 transition-colors hover:bg-cyan-600 hover:text-white"
              >
                {entry.type === 'folder' ? <FolderOpen size={14} className="shrink-0" /> : <FileCode size={14} className="shrink-0" />}
                <span className="truncate">{entry.relativePath}</span>
              </button>
            )) : (
              <p className="px-3 py-3 text-xs text-slate-400">No matching files or folders.</p>
            )}
          </div>
        )}
        <div className="ide-ai-composer">
          <textarea
            ref={inputRef}
            value={input}
            onChange={handleInputChange}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                if (mentionQuery !== null && availableMentions.length > 0) {
                  insertMention(availableMentions[0]);
                } else {
                  handleSendMessage();
                }
              }
              if (e.key === 'Escape') setMentionQuery(null);
            }}
            placeholder="Ask or write a test…"
            aria-label="Message Lumi AI"
            className="ide-ai-composer__input overflow-y-hidden text-sm"
          />
          <button
            type="button"
            onClick={isAiLoading ? stopAiResponse : handleSendMessage}
            disabled={!isAiLoading && !input.trim()}
            aria-label={isAiLoading ? 'Stop response' : 'Send message'}
            title={isAiLoading ? 'Stop response' : 'Send message (Enter)'}
            className={clsx(
              'grid h-10 w-10 place-items-center rounded-lg text-white shadow-lg transition-colors disabled:cursor-not-allowed disabled:opacity-50 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-cyan-300',
              isAiLoading ? 'bg-slate-700 hover:bg-rose-600' : 'bg-cyan-600 hover:bg-cyan-500',
            )}
          >
            {isAiLoading ? <Square size={14} fill="currentColor" /> : <Send size={18} />}
          </button>
        </div>
        <div className="mt-2 grid min-w-0 gap-1 px-1">
          <p className="flex min-w-0 items-center gap-1 text-[10px] text-slate-500">
            <Cpu size={10} className="shrink-0" />
            <span className="min-w-0 truncate">{aiConfig.provider === 'gemini'
              ? aiConfig.apiKey ? `Gemini · ${aiConfig.model}` : 'Gemini · API key required'
              : aiConfig.provider === 'codex' ? 'ChatGPT · GPT-6 Luna' : 'AGY · local'}</span>
          </p>
          <p className="min-w-0 truncate text-[10px] text-slate-600" title={`${focusedFileRelativePath || 'No focused file'} · ${openFileIds.length} open tabs · @ files/folders`}>
            {focusedFileRelativePath ? `${focusedFileRelativePath} · ` : ''}{openFileIds.length} tabs · @ files/folders
          </p>
        </div>
      </div>
      {previewProposal && (
        <div className="fixed inset-0 z-[100] bg-black/70 backdrop-blur-sm flex items-center justify-center p-6" role="dialog" aria-modal="true" aria-label="Review AI suggestion">
          <section className="w-full max-w-6xl max-h-[85vh] bg-slate-950 border border-slate-700 rounded-xl shadow-2xl flex flex-col overflow-hidden">
            <header className="px-5 py-4 border-b border-white/10 flex items-center justify-between">
              <div>
                <h2 className="text-sm font-semibold text-slate-100">Review AI suggestion</h2>
                <p className="text-xs text-slate-500 mt-1">{previewProposal.fileName} · review the complete flow replacement before applying</p>
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
              <span className="text-[11px] text-slate-500">The AI cannot write files. Applying updates the editor; auto-save writes the change to the workspace.</span>
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
