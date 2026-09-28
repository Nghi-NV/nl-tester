import React, { useState, useCallback, useEffect, useRef } from 'react';
import { useFileStore, useEditorStore, useExecutionStore, useDeviceStore, useExecutionStateStore } from '../stores';
import { FileNode } from '../types';
import {
  Folder, FolderOpen, FileCode, FilePlus2, Plus, ChevronRight, ChevronDown, Play, Loader2, Square, Check, X, Eye, EyeOff, ArrowDownAZ, CircleAlert
} from 'lucide-react';
import { clsx } from 'clsx';
import { runTestFlow } from '../services/runnerService';
import { findFileById } from '../utils/treeUtils';
import { pathJoin } from '../utils/tauriUtils';

// Drag and drop types
interface DragState {
  draggedNode: FileNode | null;
  dragOverId: string | null;
  dropPosition: 'before' | 'after' | 'inside' | null;
}

type SortMode = 'name-asc' | 'name-desc' | 'type';
type CreatingEntry = { parentId: string | null; type: 'file' | 'folder' };

const getDropPosition = (clientY: number, top: number, height: number, isFolder: boolean) => {
  if (isFolder) return 'inside' as const;
  return clientY < top + height / 2 ? 'before' as const : 'after' as const;
};

interface SidebarProps {
  width: number;
}

// Props for FileTreeItem
interface FileTreeItemProps {
  node: FileNode;
  level: number;
  parentId: string | null;
  focusedNodeId: string | null;
  setFocusedNodeId: (id: string) => void;
  // State
  activeFileId: string | null;
  runningNodeIds: string[];
  isRunning: boolean;
  hoverId: string | null;
  editingId: string | null;
  editingName: string;
  dragState: DragState;
  // Actions
  toggleFolder: (id: string) => void;
  openFile: (id: string) => void;
  deleteFile: (id: string) => void;
  stopRun: () => void;
  executeNode: (e: React.MouseEvent, node: FileNode) => void;
  setHoverId: (id: string | null) => void;
  startEditing: (node: FileNode) => void;
  onContextMenu: (event: React.MouseEvent, node: FileNode) => void;
  onCreateEntry: (type: 'file' | 'folder', target: FileNode | null) => void;
  setEditingName: (name: string) => void;
  handleRenameSubmit: (id: string) => void;
  handleRenameCancel: () => void;
  handleInputRef: (input: HTMLInputElement | null) => void;
  sortNodes: (nodes: FileNode[]) => FileNode[];
  creatingEntry: CreatingEntry | null;
  creatingName: string;
  onCreatingNameChange: (name: string) => void;
  onCommitCreate: () => void;
  onCancelCreate: () => void;
  setDragState: React.Dispatch<React.SetStateAction<DragState>>;
  handleDrop: (targetId: string, position: 'before' | 'after' | 'inside', sourceId: string) => void;
  isDescendant: (nodeId: string, potentialAncestor: FileNode) => boolean;
}

const InlineCreateEntry: React.FC<{
  type: 'file' | 'folder';
  level?: number;
  name: string;
  onNameChange: (name: string) => void;
  onCommit: () => void;
  onCancel: () => void;
}> = ({ type, level = 0, name, onNameChange, onCommit, onCancel }) => {
  const committed = useRef(false);
  return (
    <div className="ide-tree-create-row" role="treeitem" aria-label={`New ${type}`} style={{ paddingLeft: `${level * 13 + 8}px` }}>
      {type === 'folder' ? <Folder size={15} /> : <FileCode size={15} />}
      <input
        autoFocus
        value={name}
        aria-label={`${type === 'folder' ? 'Folder' : 'File'} name`}
        onFocus={event => event.currentTarget.select()}
        onChange={event => onNameChange(event.target.value)}
        onClick={event => event.stopPropagation()}
        onKeyDown={event => {
          event.stopPropagation();
          if (event.key === 'Enter') {
            event.preventDefault();
            committed.current = true;
            onCommit();
          } else if (event.key === 'Escape') {
            event.preventDefault();
            committed.current = true;
            onCancel();
          }
        }}
        onBlur={() => {
          if (!committed.current) {
            committed.current = true;
            onCommit();
          }
        }}
      />
    </div>
  );
};

// FileTreeItem as a separate component
const FileTreeItem: React.FC<FileTreeItemProps> = ({
  node, level, parentId, focusedNodeId, setFocusedNodeId,
  activeFileId, runningNodeIds, isRunning, hoverId, editingId, editingName, dragState,
  toggleFolder, openFile, deleteFile, stopRun, executeNode,
  setHoverId, startEditing, onContextMenu, onCreateEntry, setEditingName, handleRenameSubmit, handleRenameCancel,
  handleInputRef, sortNodes, creatingEntry, creatingName, onCreatingNameChange, onCommitCreate, onCancelCreate,
  setDragState, handleDrop, isDescendant
}) => {
  const isFolder = node.type === 'folder';
  const isActive = activeFileId === node.id;
  const isNodeRunning = runningNodeIds.includes(node.id);
  const isEditing = editingId === node.id;
  const isDragOver = dragState.dragOverId === node.id;
  const isDragging = dragState.draggedNode?.id === node.id;
  const folderClickTimer = useRef<number | null>(null);
  const renameCommitted = useRef(false);

  useEffect(() => {
    if (isEditing) renameCommitted.current = false;
  }, [isEditing]);

  useEffect(() => () => {
    if (folderClickTimer.current !== null) window.clearTimeout(folderClickTimer.current);
  }, []);

  // Get execution state for this file - subscribe to fileStates to trigger re-render
  const fileStates = useExecutionStateStore(state => state.fileStates);
  const fileExecutionState = isFolder ? null : fileStates.get(node.id);

  // Determine file status
  const getFileStatus = () => {
    if (!fileExecutionState) return null;

    const stepStatuses = Array.from(fileExecutionState.stepStatuses.values());
    if (stepStatuses.length === 0) return null;

    const hasRunning = stepStatuses.some(s => s === 'running') || fileExecutionState.executingStepIndex >= 0;
    const hasFailed = stepStatuses.some(s => s === 'failed');
    const hasCancelled = stepStatuses.some(s => s === 'cancelled');
    const allPassed = stepStatuses.every(s => s === 'passed') && stepStatuses.length > 0;

    if (hasRunning) return 'running';
    if (hasFailed) return 'failed';
    if (hasCancelled) return 'cancelled';
    if (allPassed) return 'passed';
    return null;
  };

  const fileStatus = getFileStatus();
  const renameInputRef = useCallback((input: HTMLInputElement | null) => {
    handleInputRef(input);
    if (!input || node.type !== 'file') return;
    const extensionStart = input.value.lastIndexOf('.');
    if (extensionStart > 0) input.setSelectionRange(0, extensionStart);
  }, [handleInputRef, node.type]);

  const handleRowClick = (event: React.MouseEvent) => {
    if (isEditing) return;
    if (!isFolder) {
      openFile(node.id);
      return;
    }
    if (event.detail > 1) {
      if (folderClickTimer.current !== null) window.clearTimeout(folderClickTimer.current);
      folderClickTimer.current = null;
      toggleFolder(node.id);
      return;
    }
    folderClickTimer.current = window.setTimeout(() => {
      folderClickTimer.current = null;
      toggleFolder(node.id);
    }, 180);
  };

  // Handle key events in edit mode
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (isEditing) {
      if (e.key === 'Enter') {
        e.preventDefault();
        if (!renameCommitted.current) {
          renameCommitted.current = true;
          handleRenameSubmit(node.id);
        }
      } else if (e.key === 'Escape') {
        renameCommitted.current = true;
        handleRenameCancel();
      }
      return;
    }
    if (e.target !== e.currentTarget) return;
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'n') {
      e.preventDefault();
      e.stopPropagation();
      onCreateEntry(e.shiftKey ? 'folder' : 'file', node);
      return;
    }
    if (e.key === 'Enter') {
      if (node.id !== 'root' && !isRunning) {
        e.preventDefault();
        startEditing(node);
      }
    } else if (e.key === 'Escape') {
      handleRenameCancel();
    } else if (e.key === 'F2' && !isEditing && node.id !== 'root' && !isRunning) {
      e.preventDefault();
      startEditing(node);
    } else if ((e.key === 'Delete' || e.key === 'Backspace') && !isEditing && node.id !== 'root' && !isRunning) {
      e.preventDefault();
      deleteFile(node.id);
    }
  };

  // Drag handlers
  const handleDragStart = (e: React.DragEvent) => {
    e.stopPropagation();
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('application/x-lumi-workspace-path', node.id);
    e.dataTransfer.setData('text/plain', node.id);
    setDragState(prev => ({ ...prev, draggedNode: node }));
  };

  const handleDragEnd = () => {
    setDragState({ draggedNode: null, dragOverId: null, dropPosition: null });
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();
    e.dataTransfer.dropEffect = 'move';

    if (!dragState.draggedNode || dragState.draggedNode.id === node.id) return;
    if (isDescendant(node.id, dragState.draggedNode)) return;

    const rect = e.currentTarget.getBoundingClientRect();
    const position = getDropPosition(e.clientY, rect.top, rect.height, isFolder);

    setDragState(prev => ({
      ...prev,
      dragOverId: node.id,
      dropPosition: position
    }));
  };

  const handleDragLeave = (e: React.DragEvent) => {
    e.preventDefault();
    const relatedTarget = e.relatedTarget as HTMLElement;
    if (!e.currentTarget.contains(relatedTarget)) {
      setDragState(prev => ({
        ...prev,
        dragOverId: prev.dragOverId === node.id ? null : prev.dragOverId,
        dropPosition: prev.dragOverId === node.id ? null : prev.dropPosition
      }));
    }
  };

  const handleDropOnItem = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();

    const sourceId = e.dataTransfer.getData('application/x-lumi-workspace-path')
      || e.dataTransfer.getData('text/plain')
      || dragState.draggedNode?.id;
    if (!sourceId) return;
    const rect = e.currentTarget.getBoundingClientRect();
    const position = getDropPosition(e.clientY, rect.top, rect.height, isFolder);
    handleDrop(node.id, position, sourceId);
  };

  const handleTreeNavigation = (event: React.KeyboardEvent<HTMLDivElement>) => {
    const rows = Array.from(document.querySelectorAll<HTMLElement>(
      '.ide-file-tree [role="treeitem"][data-node-id]',
    ));
    const currentIndex = rows.indexOf(event.currentTarget);
    const focusRow = (row?: HTMLElement) => {
      if (!row) return;
      const id = row.dataset.nodeId;
      if (id) setFocusedNodeId(id);
      row.focus();
    };

    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      const offset = event.key === 'ArrowDown' ? 1 : -1;
      focusRow(rows[Math.max(0, Math.min(rows.length - 1, currentIndex + offset))]);
    } else if (event.key === 'Home' || event.key === 'End') {
      event.preventDefault();
      focusRow(event.key === 'Home' ? rows[0] : rows[rows.length - 1]);
    } else if (event.key === 'ArrowRight' && isFolder) {
      event.preventDefault();
      if (!node.isOpen) {
        void toggleFolder(node.id);
      } else {
        const firstChild = sortNodes(node.children ?? [])[0];
        focusRow(rows.find(row => row.dataset.nodeId === firstChild?.id));
      }
    } else if (event.key === 'ArrowLeft') {
      if (isFolder && node.isOpen) {
        event.preventDefault();
        void toggleFolder(node.id);
      } else if (parentId) {
        event.preventDefault();
        focusRow(rows.find(row => row.dataset.nodeId === parentId));
      }
    } else if (event.key === ' ' || event.key === 'Spacebar') {
      event.preventDefault();
      if (isFolder) void toggleFolder(node.id);
      else openFile(node.id);
    }
  };

  // Get drop indicator styles
  const getDropIndicatorClass = () => {
    if (!isDragOver || !dragState.dropPosition) return '';

    switch (dragState.dropPosition) {
      case 'before':
        return 'border-t-2 border-cyan-500';
      case 'after':
        return 'border-b-2 border-cyan-500';
      case 'inside':
        return 'ring-2 ring-cyan-500 ring-inset bg-cyan-900/30';
      default:
        return '';
    }
  };

  return (
    <div className="select-none">
      <div
        role="treeitem"
        tabIndex={focusedNodeId === node.id ? 0 : -1}
        data-node-id={node.id}
        data-parent-id={parentId ?? undefined}
        data-level={level}
        aria-selected={isActive}
        aria-expanded={isFolder ? node.isOpen : undefined}
        className={clsx(
          "flex items-center py-1 px-2 cursor-pointer transition-colors duration-150 group relative",
          isActive ? "bg-cyan-900/30 text-cyan-400 border-l-2 border-cyan-400" : "text-slate-400 hover:bg-slate-800/50 hover:text-slate-200",
          isDragging && "opacity-50",
          getDropIndicatorClass()
        )}
        style={{ paddingLeft: `${level * 13 + (isActive ? 6 : 8)}px` }}
        onClick={handleRowClick}
        onKeyDown={event => {
          handleTreeNavigation(event);
          if (!event.defaultPrevented) handleKeyDown(event);
        }}
        onFocus={() => setFocusedNodeId(node.id)}
        onContextMenu={event => onContextMenu(event, node)}
        onMouseEnter={() => setHoverId(node.id)}
        onMouseLeave={() => setHoverId(null)}
        draggable={!isEditing && node.id !== 'root'}
        onDragStart={handleDragStart}
        onDragEnd={handleDragEnd}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onDrop={handleDropOnItem}
      >
        <span className="mr-1.5 opacity-70">
          {isFolder ? (
            node.isOpen ? <ChevronDown size={14} /> : <ChevronRight size={14} />
          ) : <span className="w-3.5" />}
        </span>

        <span className={clsx("mr-2",
          fileStatus === 'running' || isNodeRunning ? "text-amber-400" :
            fileStatus === 'failed' ? "text-rose-400" :
              fileStatus === 'cancelled' ? "text-amber-400" :
              fileStatus === 'passed' ? "text-emerald-400" :
                "text-cyan-500/80"
        )}>
          {fileStatus === 'running' || isNodeRunning ? (
            <Loader2 size={16} className="animate-spin" />
          ) : fileStatus === 'failed' ? (
            <X size={16} />
          ) : fileStatus === 'cancelled' ? (
            <CircleAlert size={16} />
          ) : fileStatus === 'passed' ? (
            <Check size={16} />
          ) : (
            isFolder ? (
              node.isOpen ? <FolderOpen size={16} /> : <Folder size={16} />
            ) : (
              <FileCode size={16} />
            )
          )}
        </span>

        {/* Name - Editable or Static */}
        {isEditing ? (
          <input
            ref={renameInputRef}
            type="text"
            value={editingName}
            onChange={(e) => setEditingName(e.target.value)}
            onKeyDown={event => { event.stopPropagation(); handleKeyDown(event); }}
            onBlur={() => {
              if (!renameCommitted.current) {
                renameCommitted.current = true;
                handleRenameSubmit(node.id);
              }
            }}
            onClick={(e) => e.stopPropagation()}
            onDoubleClick={(e) => e.stopPropagation()}
            className="text-sm flex-1 font-medium bg-slate-800 border border-cyan-500 rounded px-1 py-0.5 text-slate-100 outline-none focus:ring-1 focus:ring-cyan-400"
          />
        ) : (
          <div className="text-sm truncate flex-1 font-medium flex items-center gap-2 min-w-0">
            <span className="truncate">{node.name}</span>
            {fileExecutionState && !isFolder && (
              <span className="text-[10px] text-slate-500 flex-shrink-0 flex items-center gap-1">
                {(() => {
                  const stepStatuses = Array.from(fileExecutionState.stepStatuses.values());
                  if (stepStatuses.length === 0) return null;

                  const passedCount = stepStatuses.filter(s => s === 'passed').length;
                  const failedCount = stepStatuses.filter(s => s === 'failed').length;
                  const cancelledCount = stepStatuses.filter(s => s === 'cancelled').length;
                  const runningCount = stepStatuses.filter(s => s === 'running').length;
                  const totalSteps = stepStatuses.length;

                  const parts: string[] = [];
                  if (passedCount > 0) parts.push(`${passedCount} passed`);
                  if (failedCount > 0) parts.push(`${failedCount} failed`);
                  if (cancelledCount > 0) parts.push(`${cancelledCount} cancelled`);
                  if (runningCount > 0) parts.push(`${runningCount} running`);
                  if (totalSteps > 0) parts.push(`/ ${totalSteps} total`);

                  return parts.length > 0 ? parts.join(' ') : null;
                })()}
              </span>
            )}
          </div>
        )}

        {!isEditing && (
          <button
            type="button"
            onClick={event => {
              event.stopPropagation();
              if (isNodeRunning) stopRun();
              else void executeNode(event, node);
            }}
            className={clsx(
              'ide-explorer-run-button',
              isNodeRunning ? 'is-running' : 'is-ready',
              hoverId === node.id || isNodeRunning || isActive ? 'is-visible' : ''
            )}
            title={isNodeRunning ? 'Stop Run' : `Run ${isFolder ? 'Folder' : 'File'}`}
            aria-label={isNodeRunning ? `Stop ${node.name}` : `Run ${node.name}`}
          >
            {isNodeRunning ? <Square size={12} fill="currentColor" /> : <Play size={12} fill="currentColor" />}
          </button>
        )}
      </div>
      {isFolder && node.isOpen && node.children && (
        <div role="group">
          {creatingEntry?.parentId === node.id && (
            <InlineCreateEntry
              type={creatingEntry.type}
              level={level + 1}
              name={creatingName}
              onNameChange={onCreatingNameChange}
              onCommit={onCommitCreate}
              onCancel={onCancelCreate}
            />
          )}
          {sortNodes(node.children).map(child => (
            <FileTreeItem
              key={child.id}
              node={child}
              level={level + 1}
              parentId={node.id}
              focusedNodeId={focusedNodeId}
              setFocusedNodeId={setFocusedNodeId}
              activeFileId={activeFileId}
              runningNodeIds={runningNodeIds}
              isRunning={isRunning}
              hoverId={hoverId}
              editingId={editingId}
              editingName={editingName}
              dragState={dragState}
              toggleFolder={toggleFolder}
              openFile={openFile}
              deleteFile={deleteFile}
              stopRun={stopRun}
              executeNode={executeNode}
              setHoverId={setHoverId}
              startEditing={startEditing}
              setEditingName={setEditingName}
              handleRenameSubmit={handleRenameSubmit}
              handleRenameCancel={handleRenameCancel}
              handleInputRef={handleInputRef}
              setDragState={setDragState}
              handleDrop={handleDrop}
              isDescendant={isDescendant}
              onContextMenu={onContextMenu}
              onCreateEntry={onCreateEntry}
              sortNodes={sortNodes}
              creatingEntry={creatingEntry}
              creatingName={creatingName}
              onCreatingNameChange={onCreatingNameChange}
              onCommitCreate={onCommitCreate}
              onCancelCreate={onCancelCreate}
            />
          ))}
        </div>
      )}
    </div>
  );
};

export const Sidebar: React.FC<SidebarProps> = ({ width }) => {
  const {
    files,
    projectRoot,
    showHiddenFiles,
    setShowHiddenFiles,
    loadDescendantYamlFiles,
    addFile: addFileRaw,
    deleteFile: deleteFileRaw,
    toggleFolder,
    renameFile,
    moveFile,
  } = useFileStore();
  const { activeFileId, openFile, setActiveView } = useEditorStore();
  const { isRunning, runningNodeIds, queueRun, stopRun, setNodeRunning } = useExecutionStore();

  const deleteFile = useCallback((id: string) => {
    const node = findFileById(files, id);
    if (!node || !window.confirm(`Delete ${node.type} "${node.name}"? This cannot be undone.`)) return;
    void deleteFileRaw(id)
      .then(() => useEditorStore.getState().closeFilesWithin(id))
      .catch(error => window.alert(`Could not delete item: ${String(error)}`));
  }, [deleteFileRaw, files]);


  const [hoverId, setHoverId] = useState<string | null>(null);
  const [focusedNodeId, setFocusedNodeId] = useState<string | null>(null);
  const [contextMenu, setContextMenu] = useState<{ target: FileNode | null; x: number; y: number } | null>(null);
  const contextMenuRef = useRef<HTMLDivElement | null>(null);
  const [sortMode, setSortMode] = useState<SortMode>('name-asc');
  const [sortMenuOpen, setSortMenuOpen] = useState(false);
  const sortMenuRef = useRef<HTMLDivElement | null>(null);
  const [creatingEntry, setCreatingEntry] = useState<CreatingEntry | null>(null);
  const [creatingName, setCreatingName] = useState('');

  // Rename state
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingName, setEditingName] = useState<string>('');

  // Drag and drop state
  const [dragState, setDragState] = useState<DragState>({
    draggedNode: null,
    dragOverId: null,
    dropPosition: null
  });

  // Callback ref to auto-focus input when editing starts
  const handleInputRef = useCallback((input: HTMLInputElement | null) => {
    if (input) {
      input.focus();
      input.select();
    }
  }, []);

  // Handle rename submit
  const handleRenameSubmit = useCallback((id: string) => {
    const nextName = editingName.trim();
    const node = findFileById(files, id);
    if (nextName && node && nextName !== node.name) {
      void renameFile(id, nextName).catch(error => window.alert(`Could not rename item: ${String(error)}`));
    }
    setEditingId(null);
    setEditingName('');
  }, [editingName, files, renameFile]);

  // Handle rename cancel
  const handleRenameCancel = useCallback(() => {
    setEditingId(null);
    setEditingName('');
  }, []);

  // Start editing mode
  const startEditing = useCallback((node: FileNode) => {
    setContextMenu(null);
    setEditingId(node.id);
    setEditingName(node.name);
  }, []);

  // Find parent of a node
  const findParentId = useCallback((nodeId: string, nodes: FileNode[], parentId: string | null = null): string | null => {
    for (const node of nodes) {
      if (node.id === nodeId) return parentId;
      if (node.children) {
        const found = findParentId(nodeId, node.children, node.id);
        if (found !== undefined) return found;
      }
    }
    return null;
  }, []);

  const openContextMenu = useCallback((event: React.MouseEvent, target: FileNode | null) => {
    event.preventDefault();
    event.stopPropagation();
    setContextMenu({
      target,
      x: Math.min(event.clientX, Math.max(8, window.innerWidth - 204)),
      y: Math.min(event.clientY, Math.max(8, window.innerHeight - 184)),
    });
  }, []);

  const sortNodes = useCallback((nodes: FileNode[]) => [...nodes].sort((left, right) => {
    const folderOrder = Number(right.type === 'folder') - Number(left.type === 'folder');
    if (folderOrder) return folderOrder;
    const leftName = left.name.toLocaleLowerCase();
    const rightName = right.name.toLocaleLowerCase();
    const compareName = () => left.name.localeCompare(right.name, undefined, { numeric: true, sensitivity: 'base' });
    if (sortMode === 'type' && left.type === 'file' && right.type === 'file') {
      const extension = (name: string) => name.includes('.') ? name.slice(name.lastIndexOf('.') + 1).toLocaleLowerCase() : '';
      return extension(leftName).localeCompare(extension(rightName)) || compareName();
    }
    const order = compareName();
    return sortMode === 'name-desc' ? -order : order;
  }), [sortMode]);

  useEffect(() => {
    if (focusedNodeId && findFileById(files, focusedNodeId)) return;
    const firstNode = sortNodes(files)[0];
    if (firstNode) setFocusedNodeId(firstNode.id);
  }, [files, focusedNodeId, sortNodes]);

  const startCreateEntry = useCallback(async (type: 'file' | 'folder', target: FileNode | null) => {
    if (!projectRoot) {
      window.alert('Open a workspace before creating files or folders.');
      setContextMenu(null);
      return;
    }
    const parentId = target?.type === 'folder'
      ? target.id
      : target
        ? findParentId(target.id, files)
        : null;
    try {
      if (parentId) {
        const parentNode = findFileById(files, parentId);
        if (parentNode?.type === 'folder' && !parentNode.isOpen) await toggleFolder(parentId);
      }
      setCreatingEntry({ parentId, type });
      setCreatingName(type === 'file' ? 'test.yaml' : 'New Folder');
    } catch (error) {
      window.alert(`Could not open the target folder: ${String(error)}`);
    } finally {
      setContextMenu(null);
    }
  }, [files, findParentId, projectRoot, toggleFolder]);

  useEffect(() => {
    const handleExplorerShortcut = (event: KeyboardEvent) => {
      if (!(event.metaKey || event.ctrlKey) || event.key.toLowerCase() !== 'n') return;
      if (event.defaultPrevented) return;
      if (event.target instanceof HTMLElement && (
        event.target.isContentEditable || event.target.matches('input, textarea, select')
      )) return;
      event.preventDefault();
      const target = focusedNodeId ? findFileById(files, focusedNodeId) ?? null : null;
      void startCreateEntry(event.shiftKey ? 'folder' : 'file', target);
    };
    window.addEventListener('keydown', handleExplorerShortcut);
    return () => window.removeEventListener('keydown', handleExplorerShortcut);
  }, [files, focusedNodeId, startCreateEntry]);

  const cancelCreateEntry = useCallback(() => {
    setCreatingEntry(null);
    setCreatingName('');
  }, []);

  const commitCreateEntry = useCallback(async () => {
    if (!creatingEntry) return;
    const name = creatingName.trim();
    if (!name) {
      cancelCreateEntry();
      return;
    }
    try {
      await addFileRaw(creatingEntry.parentId, creatingEntry.type, name);
      if (creatingEntry.type === 'file' && projectRoot) {
        openFile(await pathJoin(creatingEntry.parentId || projectRoot, name));
      }
      cancelCreateEntry();
    } catch (error) {
      window.alert(`Could not create ${creatingEntry.type}: ${String(error)}`);
    }
  }, [addFileRaw, cancelCreateEntry, creatingEntry, creatingName, openFile, projectRoot]);

  useEffect(() => {
    if (!contextMenu) return;
    const closeOnOutsidePointer = (event: PointerEvent) => {
      if (event.target instanceof Node && !contextMenuRef.current?.contains(event.target)) setContextMenu(null);
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setContextMenu(null);
    };
    document.addEventListener('pointerdown', closeOnOutsidePointer);
    document.addEventListener('keydown', closeOnEscape);
    return () => {
      document.removeEventListener('pointerdown', closeOnOutsidePointer);
      document.removeEventListener('keydown', closeOnEscape);
    };
  }, [contextMenu]);

  useEffect(() => {
    if (!sortMenuOpen) return;
    const closeOnOutsidePointer = (event: PointerEvent) => {
      if (event.target instanceof Node && !sortMenuRef.current?.contains(event.target)) setSortMenuOpen(false);
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setSortMenuOpen(false);
    };
    document.addEventListener('pointerdown', closeOnOutsidePointer);
    document.addEventListener('keydown', closeOnEscape);
    return () => {
      document.removeEventListener('pointerdown', closeOnOutsidePointer);
      document.removeEventListener('keydown', closeOnEscape);
    };
  }, [sortMenuOpen]);

  // Get sibling nodes
  const getSiblings = useCallback((nodeId: string, nodes: FileNode[]): FileNode[] | null => {
    for (const node of nodes) {
      if (node.children) {
        const idx = node.children.findIndex(c => c.id === nodeId);
        if (idx !== -1) return node.children;
        const found = getSiblings(nodeId, node.children);
        if (found) return found;
      }
    }
    // Check root level
    if (nodes.find(n => n.id === nodeId)) return nodes;
    return null;
  }, []);

  // Check if node is descendant
  const isDescendant = useCallback((nodeId: string, potentialAncestor: FileNode): boolean => {
    if (potentialAncestor.id === nodeId) return true;
    if (potentialAncestor.children) {
      return potentialAncestor.children.some(child => isDescendant(nodeId, child));
    }
    return false;
  }, []);

  // Handle drop
  const handleDrop = useCallback(async (targetId: string, position: 'before' | 'after' | 'inside', sourceId: string) => {
    const draggedNode = findFileById(files, sourceId) ?? dragState.draggedNode;
    if (!draggedNode || draggedNode.id === targetId) return;

    // Check if trying to drop into itself or its descendants
    if (isDescendant(targetId, draggedNode)) return;

    const sourceParentId = findParentId(draggedNode.id, files);
    const target = findFileById(files, targetId);
    if (position === 'inside' && target?.type !== 'folder') return;

    const targetParentId = findParentId(targetId, files);
    const siblings = position === 'inside' ? null : getSiblings(targetId, files);
    if (position !== 'inside' && !siblings) return;

    if ((position === 'inside' && sourceParentId === targetId)
      || (position !== 'inside' && sourceParentId === targetParentId)) {
      setDragState({ draggedNode: null, dragOverId: null, dropPosition: null });
      return;
    }

    const targetIndex = siblings?.findIndex(s => s.id === targetId) ?? -1;

    try {
      if (position === 'inside') {
        await moveFile(draggedNode.id, targetId, 0);
        if (target?.type === 'folder' && !target.isOpen) await toggleFolder(targetId);
      } else if (position === 'before') {
        await moveFile(draggedNode.id, targetParentId, targetIndex);
      } else {
        await moveFile(draggedNode.id, targetParentId, targetIndex + 1);
      }
    } catch (error) {
      window.alert(`Could not move item: ${String(error)}`);
    } finally {
      setDragState({ draggedNode: null, dragOverId: null, dropPosition: null });
    }
  }, [dragState, files, findParentId, getSiblings, isDescendant, moveFile, toggleFolder]);

  const executeNode = async (e: React.MouseEvent, node: FileNode) => {
    e.stopPropagation();
    const { selectedPlatform, selectedDevice } = useDeviceStore.getState();
    const runLabel = `${node.name} · ${node.type === 'folder' ? 'Run folder' : 'Run test'}`;

    try {
      await queueRun(runLabel, async ({ signal, runId }) => {
      const batchId = node.type === 'folder' ? runId : undefined;

      try {
        const filesToRun = await loadDescendantYamlFiles(node.id);
        if (node.type === 'folder') setNodeRunning(node.id, true);

        for (const file of filesToRun) {
          if (signal.aborted) break;
          if (!file.content) continue;

          setNodeRunning(file.id, true);
          setNodeRunning(node.id, true);

          try {
            const result = await runTestFlow(
              file.content,
              file.id,
              file.name,
              selectedPlatform,
              selectedDevice,
              partial => {
                if (partial.id) useExecutionStore.getState().upsertResult(partial as any);
                const runningStep = [...(partial.steps || [])].reverse().find(step => step.status === 'running');
                if (runningStep) useEditorStore.getState().setActiveStepName(runningStep.name);
              },
              signal,
            );

            if (batchId) {
              result.batchId = batchId;
              result.folderName = node.name;
            }

            useExecutionStore.getState().upsertResult(result);
            setActiveView('report');
          } catch (error) {
            console.error(error);
          } finally {
            setNodeRunning(file.id, false);
          }
        }
      } finally {
        if (node.type === 'folder') setNodeRunning(node.id, false);
        if (node.type === 'file') setNodeRunning(node.id, false);
      }
      });
    } catch (error) {
      window.alert(`Could not queue ${node.name}: ${String(error)}`);
    }
  };

  return (
    <div className="ide-explorer-panel h-full flex flex-col" style={{ width, flex: `0 0 ${width}px` }} onContextMenu={event => openContextMenu(event, null)}>
      <div className="ide-explorer-header p-4 border-b border-borderGlass flex items-center justify-between">
        <h2 className="text-sm font-bold text-slate-100 tracking-wider flex items-center gap-2 min-w-0">
          <span className="truncate">EXPLORER</span>
        </h2>
        <div className="flex items-center gap-1">
          <div className="ide-explorer-sort" ref={sortMenuRef}>
            <button
              type="button"
              onClick={() => setSortMenuOpen(open => !open)}
              className="text-slate-500 hover:text-cyan-400 transition-colors p-1"
              title={sortMode === 'name-desc' ? 'Sorted by name, Z to A' : sortMode === 'type' ? 'Sorted by file type' : 'Sorted by name, A to Z'}
              aria-label="Sort workspace files"
              aria-expanded={sortMenuOpen}
            >
              <ArrowDownAZ size={16} />
            </button>
            {sortMenuOpen && (
              <div className="ide-explorer-sort-menu" role="menu" aria-label="Sort files">
                <button role="menuitemradio" aria-checked={sortMode === 'name-asc'} onClick={() => { setSortMode('name-asc'); setSortMenuOpen(false); }}>Name (A to Z)</button>
                <button role="menuitemradio" aria-checked={sortMode === 'name-desc'} onClick={() => { setSortMode('name-desc'); setSortMenuOpen(false); }}>Name (Z to A)</button>
                <button role="menuitemradio" aria-checked={sortMode === 'type'} onClick={() => { setSortMode('type'); setSortMenuOpen(false); }}>File type</button>
              </div>
            )}
          </div>
          <button
            onClick={async () => {
              try {
                const { openDialog } = await import('../utils/tauriUtils');
                const selected = await openDialog({
                  directory: true,
                  multiple: false,
                });
                if (selected && typeof selected === 'string') {
                  const fileStore = useFileStore.getState();
                  if (fileStore.dirtyFileIds.length > 0) await fileStore.saveAllFiles();
                  await fileStore.loadProject(selected);
                }
              } catch (err) {
                window.alert(`Could not open workspace: ${String(err)}`);
              }
            }}
            className="text-slate-500 hover:text-cyan-400 transition-colors p-1"
            title="Open Project Folder"
          >
            <FolderOpen size={16} />
          </button>
          <button
            onClick={() => void setShowHiddenFiles(!showHiddenFiles)}
            className="text-slate-500 hover:text-cyan-400 transition-colors p-1"
            title={showHiddenFiles ? 'Hide hidden files' : 'Show hidden files'}
          >
            {showHiddenFiles ? <Eye size={16} /> : <EyeOff size={16} />}
          </button>
          <button
            onClick={() => void startCreateEntry('file', null)}
            className="text-slate-500 hover:text-cyan-400 transition-colors p-1"
            title="New File (⌘N)"
          >
            <FilePlus2 size={16} />
          </button>
          <button
            onClick={() => void startCreateEntry('folder', null)}
            className="text-slate-500 hover:text-cyan-400 transition-colors p-1"
            title="New Folder (⌘⇧N)"
          >
            <Plus size={16} />
          </button>
        </div>
      </div>

      <div className="ide-file-tree flex-1 overflow-y-auto py-2" role="tree" aria-label="Workspace files">
        {creatingEntry?.parentId === null && (
          <InlineCreateEntry
            type={creatingEntry.type}
            name={creatingName}
            onNameChange={setCreatingName}
            onCommit={() => void commitCreateEntry()}
            onCancel={cancelCreateEntry}
          />
        )}
        {sortNodes(files).map(node => (
          <FileTreeItem
            key={node.id}
            node={node}
            level={0}
            parentId={null}
            focusedNodeId={focusedNodeId}
            setFocusedNodeId={setFocusedNodeId}
            activeFileId={activeFileId}
            runningNodeIds={runningNodeIds}
            isRunning={isRunning}
            hoverId={hoverId}
            editingId={editingId}
            editingName={editingName}
            dragState={dragState}
            toggleFolder={toggleFolder}
            openFile={openFile}
            deleteFile={deleteFile}
            stopRun={stopRun}
            executeNode={executeNode}
            setHoverId={setHoverId}
            startEditing={startEditing}
            setEditingName={setEditingName}
            handleRenameSubmit={handleRenameSubmit}
            handleRenameCancel={handleRenameCancel}
            handleInputRef={handleInputRef}
            setDragState={setDragState}
            handleDrop={handleDrop}
            isDescendant={isDescendant}
            onContextMenu={openContextMenu}
            onCreateEntry={startCreateEntry}
            sortNodes={sortNodes}
            creatingEntry={creatingEntry}
            creatingName={creatingName}
            onCreatingNameChange={setCreatingName}
            onCommitCreate={() => void commitCreateEntry()}
            onCancelCreate={cancelCreateEntry}
          />
        ))}
      </div>
      {contextMenu && (
        <div
          ref={contextMenuRef}
          className="ide-explorer-context-menu"
          role="menu"
          style={{ left: contextMenu.x, top: contextMenu.y }}
          onContextMenu={event => event.preventDefault()}
        >
          {contextMenu.target?.type === 'file' && (
            <button role="menuitem" onClick={() => { openFile(contextMenu.target!.id); setContextMenu(null); }}>
              Open
            </button>
          )}
          <button role="menuitem" onClick={() => void startCreateEntry('file', contextMenu.target)}>New File…</button>
          <button role="menuitem" onClick={() => void startCreateEntry('folder', contextMenu.target)}>New Folder…</button>
          {contextMenu.target && contextMenu.target.id !== 'root' && <>
            <div className="ide-context-menu-divider" />
            <button role="menuitem" onClick={() => startEditing(contextMenu.target!)}>Rename</button>
            <button role="menuitem" className="is-danger" onClick={() => { deleteFile(contextMenu.target!.id); setContextMenu(null); }}>Delete</button>
          </>}
        </div>
      )}
    </div>
  );
};
