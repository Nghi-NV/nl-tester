import { create } from 'zustand';
import { FileNode } from '../types';
import { generateId } from '../utils/idGenerator';
import { getInitialFiles } from '../utils/initialData';
import {
  addNodeToTree,
  deleteNodeFromTree,
  updateFileContentInTree,
  toggleFolderInTree,
  findFileByName,
  renameNodeInTree,
  moveNodeInTree,
} from '../utils/treeUtils';
import { FILE_CONFIG } from '../constants';

interface FileStore {
  files: FileNode[];

  // Actions
  addFile: (parentId: string | null, type: 'file' | 'folder', name: string) => void;
  deleteFile: (id: string) => void;
  updateFileContent: (id: string, content: string) => void;
  toggleFolder: (id: string) => void;
  renameFile: (id: string, newName: string) => void;
  moveFile: (id: string, newParentId: string | null, index: number) => void;
  getFileContentByName: (name: string) => string | null;
}

export const useFileStore = create<FileStore>((set, get) => ({
  files: getInitialFiles(),

  addFile: (parentId, type, name) => {
    const newId = generateId();
    const newFile: FileNode = {
      id: newId,
      name,
      type,
      children: type === 'folder' ? [] : undefined,
      content: type === 'file' ? FILE_CONFIG.DEFAULT_FILE_CONTENT : undefined,
      isOpen: true
    };

    const updatedFiles = addNodeToTree(get().files, parentId, newFile);
    set({ files: updatedFiles });
  },

  deleteFile: (id) => {
    set({ files: deleteNodeFromTree(get().files, id) });
  },

  updateFileContent: (id, content) => {
    set({ files: updateFileContentInTree(get().files, id, content) });
  },

  toggleFolder: (id) => {
    set({ files: toggleFolderInTree(get().files, id) });
  },

  renameFile: (id, newName) => {
    set({ files: renameNodeInTree(get().files, id, newName) });
  },

  moveFile: (id, newParentId, index) => {
    set({ files: moveNodeInTree(get().files, id, newParentId, index) });
  },

  getFileContentByName: (name: string) => {
    const file = findFileByName(get().files, name);
    return file?.content || null;
  },
}));

