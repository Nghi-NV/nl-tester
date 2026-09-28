import { create } from 'zustand';
import { ViewMode } from '../types';

export interface EditorRevealPosition {
  fileId: string;
  lineNumber: number;
  column: number;
}

interface EditorStore {
  activeFileId: string | null;
  openFiles: string[];
  activeView: ViewMode;
  activeStepName: string | null; // For editor highlighting
  pendingReveal: EditorRevealPosition | null;

  // Actions
  openFile: (id: string) => void;
  openFileAt: (id: string, lineNumber: number, column: number) => void;
  clearPendingReveal: (fileId: string) => void;
  closeFile: (id: string) => void;
  closeAllFiles: () => void;
  closeFilesWithin: (path: string) => void;
  replaceFilePath: (oldPath: string, newPath: string) => void;
  setActiveFile: (id: string) => void;
  setActiveView: (view: ViewMode) => void;
  setActiveStepName: (name: string | null) => void;
}

export const useEditorStore = create<EditorStore>((set, get) => ({
  activeFileId: null,
  openFiles: [],
  activeView: 'editor',
  activeStepName: null,
  pendingReveal: null,

  openFile: (id) => {
    const { openFiles } = get();
    if (!openFiles.includes(id)) {
      set({ openFiles: [...openFiles, id], activeFileId: id, activeView: 'editor', pendingReveal: null });
    } else {
      set({ activeFileId: id, activeView: 'editor', pendingReveal: null });
    }
  },

  openFileAt: (id, lineNumber, column) => set(state => ({
    openFiles: state.openFiles.includes(id) ? state.openFiles : [...state.openFiles, id],
    activeFileId: id,
    activeView: 'editor',
    pendingReveal: { fileId: id, lineNumber, column },
  })),

  clearPendingReveal: fileId => set(state => state.pendingReveal?.fileId === fileId
    ? { pendingReveal: null }
    : {}),

  closeFile: (id) => {
    const { openFiles, activeFileId } = get();
    const newOpenFiles = openFiles.filter(fid => fid !== id);
    let newActiveId = activeFileId;
    if (activeFileId === id) {
      newActiveId = newOpenFiles.length > 0 ? newOpenFiles[newOpenFiles.length - 1] : null;
    }
    set({ openFiles: newOpenFiles, activeFileId: newActiveId });
  },

  closeAllFiles: () => set({ activeFileId: null, openFiles: [], activeStepName: null, pendingReveal: null }),

  closeFilesWithin: path => set(state => {
    const contains = (filePath: string) => filePath === path
      || filePath.startsWith(`${path}/`)
      || filePath.startsWith(`${path}\\`);
    const openFiles = state.openFiles.filter(filePath => !contains(filePath));
    return {
      openFiles,
      activeFileId: state.activeFileId && !contains(state.activeFileId)
        ? state.activeFileId
        : openFiles[openFiles.length - 1] ?? null,
      pendingReveal: state.pendingReveal && contains(state.pendingReveal.fileId) ? null : state.pendingReveal,
    };
  }),

  replaceFilePath: (oldPath, newPath) => set(state => {
    const replace = (path: string) => path === oldPath || path.startsWith(`${oldPath}/`) || path.startsWith(`${oldPath}\\`)
      ? `${newPath}${path.slice(oldPath.length)}`
      : path;
    return {
      openFiles: state.openFiles.map(replace),
      activeFileId: state.activeFileId ? replace(state.activeFileId) : null,
    };
  }),

  setActiveFile: (id) => set({ activeFileId: id }),
  setActiveView: (view) => set({ activeView: view }),
  setActiveStepName: (name) => set({ activeStepName: name }),
}));
