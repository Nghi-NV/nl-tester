import { create } from 'zustand';
import { ViewMode } from '../types';

interface EditorStore {
  activeFileId: string | null;
  openFiles: string[];
  activeView: ViewMode;
  activeStepName: string | null; // For editor highlighting

  // Actions
  openFile: (id: string) => void;
  closeFile: (id: string) => void;
  setActiveFile: (id: string) => void;
  setActiveView: (view: ViewMode) => void;
  setActiveStepName: (name: string | null) => void;
}

export const useEditorStore = create<EditorStore>((set, get) => ({
  activeFileId: '1',
  openFiles: ['1'],
  activeView: 'editor',
  activeStepName: null,

  openFile: (id) => {
    const { openFiles } = get();
    if (!openFiles.includes(id)) {
      set({ openFiles: [...openFiles, id], activeFileId: id, activeView: 'editor' });
    } else {
      set({ activeFileId: id, activeView: 'editor' });
    }
  },

  closeFile: (id) => {
    const { openFiles, activeFileId } = get();
    const newOpenFiles = openFiles.filter(fid => fid !== id);
    let newActiveId = activeFileId;
    if (activeFileId === id) {
      newActiveId = newOpenFiles.length > 0 ? newOpenFiles[newOpenFiles.length - 1] : null;
    }
    set({ openFiles: newOpenFiles, activeFileId: newActiveId });
  },

  setActiveFile: (id) => set({ activeFileId: id }),
  setActiveView: (view) => set({ activeView: view }),
  setActiveStepName: (name) => set({ activeStepName: name }),
}));

