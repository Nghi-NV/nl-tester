import React, { useState, useCallback } from 'react';
import { useFileStore, useEditorStore, useExecutionStore, useDeviceStore, getAllDescendantFiles, useExecutionStateStore } from '../stores';
import { FileNode } from '../types';
import {
  Folder, FolderOpen, FileCode, Plus, Trash2, ChevronRight, ChevronDown, Play, Loader2, Square, Pencil, Check, X
} from 'lucide-react';
import { clsx } from 'clsx';
import { runTestFlow } from '../services/runnerService';
import { generateRunId } from '../utils/idGenerator';

// Drag and drop types
interface DragState {
  draggedNode: FileNode | null;
  dragOverId: string | null;
  dropPosition: 'before' | 'after' | 'inside' | null;
}

// Props for FileTreeItem
interface FileTreeItemProps {
  node: FileNode;
  level: number;
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
  addFile: (parentId: string | null, type: 'file' | 'folder', name: string) => void;
  deleteFile: (id: string) => void;
  stopRun: () => void;
  executeNode: (e: React.MouseEvent, node: FileNode) => void;
  setHoverId: (id: string | null) => void;
  startEditing: (node: FileNode) => void;
  setEditingName: (name: string) => void;
  handleRenameSubmit: (id: string) => void;
  handleRenameCancel: () => void;
  handleInputRef: (input: HTMLInputElement | null) => void;
  setDragState: React.Dispatch<React.SetStateAction<DragState>>;
  handleDrop: (targetId: string, position: 'before' | 'after' | 'inside') => void;
  isDescendant: (nodeId: string, potentialAncestor: FileNode) => boolean;
}

// FileTreeItem as a separate component
const FileTreeItem: React.FC<FileTreeItemProps> = ({
  node, level,
  activeFileId, runningNodeIds, isRunning, hoverId, editingId, editingName, dragState,
  toggleFolder, openFile, addFile, deleteFile, stopRun, executeNode,
  setHoverId, startEditing, setEditingName, handleRenameSubmit, handleRenameCancel,
  handleInputRef, setDragState, handleDrop, isDescendant
}) => {
  const isFolder = node.type === 'folder';
  const isActive = activeFileId === node.id;
  const isNodeRunning = runningNodeIds.includes(node.id);
  const isEditing = editingId === node.id;
  const isDragOver = dragState.dragOverId === node.id;
  const isDragging = dragState.draggedNode?.id === node.id;

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
    const allPassed = stepStatuses.every(s => s === 'passed') && stepStatuses.length > 0;

    if (hasRunning) return 'running';
    if (hasFailed) return 'failed';
    if (allPassed) return 'passed';
    return null;
  };

  const fileStatus = getFileStatus();

  // Handle double-click to rename
  const handleDoubleClick = (e: React.MouseEvent) => {
    e.stopPropagation();
    e.preventDefault();
    if (node.id !== 'root' && !isRunning) {
      startEditing(node);
    }
  };

  // Handle key events in edit mode
  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      handleRenameSubmit(node.id);
    } else if (e.key === 'Escape') {
      handleRenameCancel();
    }
  };

  // Drag handlers
  const handleDragStart = (e: React.DragEvent) => {
    e.stopPropagation();
    e.dataTransfer.effectAllowed = 'move';
    e.dataTransfer.setData('text/plain', node.id);
    setDragState(prev => ({ ...prev, draggedNode: node }));
  };

  const handleDragEnd = () => {
    setDragState({ draggedNode: null, dragOverId: null, dropPosition: null });
  };

  const handleDragOver = (e: React.DragEvent) => {
    e.preventDefault();
    e.stopPropagation();

    if (!dragState.draggedNode || dragState.draggedNode.id === node.id) return;
    if (isDescendant(node.id, dragState.draggedNode)) return;

    const rect = e.currentTarget.getBoundingClientRect();
    const y = e.clientY - rect.top;
    const height = rect.height;

    let position: 'before' | 'after' | 'inside';

    if (isFolder) {
      // For folders, divide into 3 zones
      if (y < height * 0.25) {
        position = 'before';
      } else if (y > height * 0.75) {
        position = 'after';
      } else {
        position = 'inside';
      }
    } else {
      // For files, only before/after
      position = y < height / 2 ? 'before' : 'after';
    }

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

    if (dragState.dropPosition) {
      handleDrop(node.id, dragState.dropPosition);
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
        className={clsx(
          "flex items-center py-1 px-2 cursor-pointer transition-colors duration-150 group relative",
          isActive ? "bg-cyan-900/30 text-cyan-400 border-l-2 border-cyan-400" : "text-slate-400 hover:bg-slate-800/50 hover:text-slate-200",
          isDragging && "opacity-50",
          getDropIndicatorClass()
        )}
        style={{ paddingLeft: `${level * 13 + (isActive ? 6 : 8)}px` }}
        onClick={() => !isEditing && (isFolder ? toggleFolder(node.id) : openFile(node.id))}
        onDoubleClick={handleDoubleClick}
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
              fileStatus === 'passed' ? "text-emerald-400" :
                "text-cyan-500/80"
        )}>
          {fileStatus === 'running' || isNodeRunning ? (
            <Loader2 size={16} className="animate-spin" />
          ) : fileStatus === 'failed' ? (
            <X size={16} />
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
            ref={handleInputRef}
            type="text"
            value={editingName}
            onChange={(e) => setEditingName(e.target.value)}
            onKeyDown={handleKeyDown}
            onBlur={() => handleRenameSubmit(node.id)}
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
                  const runningCount = stepStatuses.filter(s => s === 'running').length;
                  const totalSteps = stepStatuses.length;

                  const parts: string[] = [];
                  if (passedCount > 0) parts.push(`${passedCount} passed`);
                  if (failedCount > 0) parts.push(`${failedCount} failed`);
                  if (runningCount > 0) parts.push(`${runningCount} running`);
                  if (totalSteps > 0) parts.push(`/ ${totalSteps} total`);

                  return parts.length > 0 ? parts.join(' ') : null;
                })()}
              </span>
            )}
          </div>
        )}

        {/* Actions */}
        {!isEditing && (
          <div className={clsx("flex items-center gap-1 transition-opacity absolute right-2 bg-slate-900/80 backdrop-blur rounded px-1", hoverId === node.id || isNodeRunning ? "opacity-100" : "opacity-0 pointer-events-none")}>

            {/* Rename Button */}
            {node.id !== 'root' && !isRunning && (
              <button
                onClick={(e) => { e.stopPropagation(); startEditing(node); }}
                className="p-1 hover:bg-slate-700 rounded text-slate-300" title="Rename"
              >
                <Pencil size={12} />
              </button>
            )}

            {/* Stop Button if running, Play if not */}
            {isRunning && isNodeRunning ? (
              <button
                onClick={(e) => { e.stopPropagation(); stopRun(); }}
                className="p-1 hover:bg-rose-900/50 text-rose-400 rounded"
                title="Stop Execution"
              >
                <Square size={12} fill="currentColor" />
              </button>
            ) : (
              !isRunning && (
                <button
                  onClick={(e) => executeNode(e, node)}
                  className="p-1 hover:bg-emerald-900/50 text-emerald-400 rounded"
                  title={`Run ${isFolder ? 'Folder' : 'File'}`}
                >
                  <Play size={12} fill="currentColor" />
                </button>
              )
            )}

            {isFolder && !isRunning && (
              <>
                <button
                  onClick={(e) => { e.stopPropagation(); addFile(node.id, 'file', `test.yaml`); }}
                  className="p-1 hover:bg-slate-700 rounded text-slate-300" title="Add File"
                >
                  <Plus size={12} />
                </button>
                <button
                  onClick={(e) => { e.stopPropagation(); addFile(node.id, 'folder', 'New Folder'); }}
                  className="p-1 hover:bg-slate-700 rounded text-slate-300" title="Add Folder"
                >
                  <Folder size={12} />
                </button>
              </>
            )}
            {node.id !== 'root' && !isRunning && (
              <button
                onClick={(e) => { e.stopPropagation(); deleteFile(node.id); }}
                className="p-1 hover:bg-red-900/50 text-red-400 rounded" title="Delete"
              >
                <Trash2 size={12} />
              </button>
            )}
          </div>
        )}
      </div>
      {isFolder && node.isOpen && node.children && (
        <div>
          {node.children.map(child => (
            <FileTreeItem
              key={child.id}
              node={child}
              level={level + 1}
              activeFileId={activeFileId}
              runningNodeIds={runningNodeIds}
              isRunning={isRunning}
              hoverId={hoverId}
              editingId={editingId}
              editingName={editingName}
              dragState={dragState}
              toggleFolder={toggleFolder}
              openFile={openFile}
              addFile={addFile}
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
            />
          ))}
        </div>
      )}
    </div>
  );
};

export const Sidebar: React.FC = () => {
  const { files, addFile: addFileRaw, deleteFile, toggleFolder, renameFile, moveFile } = useFileStore();
  const { activeFileId, openFile, setActiveView } = useEditorStore();
  const { isRunning, runningNodeIds, startRun, stopRun, setNodeRunning, addResult } = useExecutionStore();


  const [hoverId, setHoverId] = useState<string | null>(null);

  // Rename state
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingName, setEditingName] = useState<string>('');

  // Drag and drop state
  const [dragState, setDragState] = useState<DragState>({
    draggedNode: null,
    dragOverId: null,
    dropPosition: null
  });

  // Enhanced addFile that also opens the new file
  const addFile = useCallback((parentId: string | null, type: 'file' | 'folder', name: string) => {
    addFileRaw(parentId, type, name);
    if (type === 'file') {
      // Find the newly created file and open it
      setTimeout(() => {
        const updatedFiles = useFileStore.getState().files;
        const findNewFile = (nodes: FileNode[]): FileNode | null => {
          for (const node of nodes) {
            if (node.name === name && node.type === type) return node;
            if (node.children) {
              const found = findNewFile(node.children);
              if (found) return found;
            }
          }
          return null;
        };
        const newFile = findNewFile(updatedFiles);
        if (newFile) {
          useEditorStore.getState().openFile(newFile.id);
        }
      }, 0);
    }
  }, [addFileRaw]);

  // Callback ref to auto-focus input when editing starts
  const handleInputRef = useCallback((input: HTMLInputElement | null) => {
    if (input) {
      input.focus();
      input.select();
    }
  }, []);

  // Handle rename submit
  const handleRenameSubmit = useCallback((id: string) => {
    if (editingName.trim()) {
      renameFile(id, editingName.trim());
    }
    setEditingId(null);
    setEditingName('');
  }, [editingName, renameFile]);

  // Handle rename cancel
  const handleRenameCancel = useCallback(() => {
    setEditingId(null);
    setEditingName('');
  }, []);

  // Start editing mode
  const startEditing = useCallback((node: FileNode) => {
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
    return undefined as unknown as string | null;
  }, []);

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
  const handleDrop = useCallback((targetId: string, position: 'before' | 'after' | 'inside') => {
    const { draggedNode } = dragState;
    if (!draggedNode || draggedNode.id === targetId) return;

    // Check if trying to drop into itself or its descendants
    if (isDescendant(targetId, draggedNode)) return;

    const targetParentId = findParentId(targetId, files);
    const siblings = getSiblings(targetId, files);

    if (!siblings) return;

    const targetIndex = siblings.findIndex(s => s.id === targetId);

    if (position === 'inside') {
      // Move inside a folder
      moveFile(draggedNode.id, targetId, 0);
    } else if (position === 'before') {
      moveFile(draggedNode.id, targetParentId, targetIndex);
    } else {
      moveFile(draggedNode.id, targetParentId, targetIndex + 1);
    }

    setDragState({ draggedNode: null, dragOverId: null, dropPosition: null });
  }, [dragState, files, findParentId, getSiblings, isDescendant, moveFile]);

  const executeNode = async (e: React.MouseEvent, node: FileNode) => {
    e.stopPropagation();

    if (isRunning) return;

    const signal = startRun();

    // Identify files to run
    const filesToRun = getAllDescendantFiles(node);

    // Generate Batch ID if running a folder (multiple files)
    const batchId = node.type === 'folder' ? generateRunId() : undefined;
    const batchFolderName = node.type === 'folder' ? node.name : undefined;

    if (node.type === 'folder') setNodeRunning(node.id, true);

    try {
      for (const file of filesToRun) {
        if (signal.aborted) break;
        if (!file.content) continue;

        setNodeRunning(file.id, true);
        useExecutionStore.getState().setNodeRunning(node.id, true);

        // Create a placeholder result to show immediately?
        // runTestFlow creates it internally, but we get updates via callback.

        try {
          const result = await runTestFlow(
            file.content,
            file.id,
            file.name,
            useDeviceStore.getState().selectedPlatform,
            useDeviceStore.getState().selectedDevice,
            (partial) => {
              // Live update
              if (partial.id) {
                useExecutionStore.getState().upsertResult(partial as any);
              }
              // Sync active step for editor highlighting
              const runningStep = [...(partial.steps || [])].reverse().find(s => s.status === 'running');
              if (runningStep) {
                useEditorStore.getState().setActiveStepName(runningStep.name);
              }
            },
            signal
          );

          // Final update or add (upsert handles both)
          useExecutionStore.getState().upsertResult(result);

          if (batchId) {
            result.batchId = batchId;
            if (batchFolderName) {
              result.folderName = batchFolderName;
            }
          }

          addResult(result);
          
          // Tự động chuyển sang tab Reports sau khi test chạy xong
          setActiveView('report');
        } catch (error) {
          console.error(error);
        }

        setNodeRunning(file.id, false);
      }
    } finally {
      if (node.type === 'folder') setNodeRunning(node.id, false);
      stopRun(); // Ensure cleanup
    }
  };

  return (
    <div className="w-64 h-full bg-slate-950 border-r border-borderGlass flex flex-col">
      <div className="p-4 border-b border-borderGlass flex items-center justify-between">
        <h2 className="text-sm font-bold text-slate-100 tracking-wider flex items-center gap-2">
          <span className="w-2 h-2 rounded-full bg-cyan-500 animate-pulse"></span>
          EXPLORER
        </h2>
        <div className="flex items-center gap-1">
          <button
            onClick={async () => {
              try {
                const { openDialog } = await import('../utils/tauriUtils');
                const selected = await openDialog({
                  directory: true,
                  multiple: false,
                });
                if (selected && typeof selected === 'string') {
                  useFileStore.getState().loadProject(selected);
                }
              } catch (err) {
                console.error("Failed to open project", err);
              }
            }}
            className="text-slate-500 hover:text-cyan-400 transition-colors p-1"
            title="Open Project Folder"
          >
            <FolderOpen size={16} />
          </button>
          <button
            onClick={() => addFile(null, 'folder', 'New Folder')}
            className="text-slate-500 hover:text-cyan-400 transition-colors p-1"
            title="New Folder"
          >
            <Plus size={16} />
          </button>
        </div>
      </div>


      <div className="flex-1 overflow-y-auto py-2">
        {files.map(node => (
          <FileTreeItem
            key={node.id}
            node={node}
            level={0}
            activeFileId={activeFileId}
            runningNodeIds={runningNodeIds}
            isRunning={isRunning}
            hoverId={hoverId}
            editingId={editingId}
            editingName={editingName}
            dragState={dragState}
            toggleFolder={toggleFolder}
            openFile={openFile}
            addFile={addFile}
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
          />
        ))}
      </div>
    </div>
  );
};
