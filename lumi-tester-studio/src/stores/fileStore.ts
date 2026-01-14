
import { create } from 'zustand';
import { FileNode } from '../types';
import {
  updateFileContentInTree,
  toggleFolderInTree,
  findFileByName,
} from '../utils/treeUtils';
import { FILE_CONFIG } from '../constants';

import {
  readDir,
  readFile,
  writeFile,
  createDir,
  deletePath,
  renamePath,
  pathJoin
} from '../utils/tauriUtils';

interface FileStore {
  files: FileNode[];
  projectRoot: string | null;
  isLoading: boolean;

  // Actions
  loadProject: (path: string) => Promise<void>;
  addFile: (parentId: string | null, type: 'file' | 'folder', name: string) => Promise<void>;
  deleteFile: (id: string) => Promise<void>;
  updateFileContent: (id: string, content: string) => Promise<void>;
  toggleFolder: (id: string) => void;
  renameFile: (id: string, newName: string) => Promise<void>;
  moveFile: (id: string, newParentId: string | null, index: number) => Promise<void>; // Move is complex for FS, might implement basic version
  getFileContentByName: (name: string) => string | null;
  loadContent: (id: string) => Promise<void>; // New action
  refresh: () => Promise<void>;
}

// Helper to build tree from FS
const buildFileTree = async (path: string, rootPath: string): Promise<FileNode[]> => {
  const entries = await readDir(path);
  const nodes: FileNode[] = [];

  // Sort: folders first, then files
  entries.sort((a: any, b: any) => {
    if (a.isDirectory === b.isDirectory) return a.name.localeCompare(b.name);
    return a.isDirectory ? -1 : 1;
  });

  console.log(`[FileStore] Loaded ${entries.length} entries for ${path}`);

  for (const entry of entries) {
    // Skip hidden files/dirs if needed, e.g. .git
    if (entry.name.startsWith('.')) continue;

    const fullPath = await pathJoin(path, entry.name);

    // Using full path as ID simplifies FS ops
    const id = fullPath;

    let children: FileNode[] | undefined;
    let content: string | undefined;

    if (entry.isDirectory) {
      children = await buildFileTree(fullPath, rootPath);
    } else {
      // We don't load content eagerly for all files to be fast, 
      // but current architecture expects content in node for simple access.
      // For a large project, this is bad. For now, let's lazy load or load on demand.
      // Existing app expects content allowed. Let's load content on openFile in editor store instead?
      // But existing execution model relies on content in store?
      // "runTestFlow" takes content string.

      // Compromise: Don't load content here. Load it when opening or running.
      // We'll require async `getFileContent` or update store when file opens.
      content = undefined;
    }

    nodes.push({
      id,
      name: entry.name,
      type: entry.isDirectory ? 'folder' : 'file',
      children,
      content,
      isOpen: false
    });
  }
  return nodes;
};

export const useFileStore = create<FileStore>((set, get) => ({
  files: [],
  projectRoot: null,
  isLoading: false,

  loadProject: async (path: string) => {
    set({ isLoading: true });
    try {
      const files = await buildFileTree(path, path);
      set({ files, projectRoot: path });
      localStorage.setItem('lumi_project_root', path);
    } catch (e) {
      console.error('Failed to load project', e);
    } finally {
      set({ isLoading: false });
    }
  },

  refresh: async () => {
    const root = get().projectRoot;
    if (root) {
      await get().loadProject(root);
    }
  },

  addFile: async (parentId, type, name) => {
    // parentId is now the absolute path of the directory
    // If parentId is null, use projectRoot
    const root = get().projectRoot;
    if (!root) return;

    const parentPath = parentId || root;
    const newPath = await pathJoin(parentPath, name);

    try {
      if (type === 'folder') {
        await createDir(newPath);
      } else {
        // Check if file exists?
        await writeFile(newPath, FILE_CONFIG.DEFAULT_FILE_CONTENT || '');
      }
      // Refresh tree to keep simple sync
      await get().refresh();

      // Locate new file to set open/active? handled by caller via side effect usually involves ID.
    } catch (e) {
      console.error('Failed to add file', e);
    }
  },

  deleteFile: async (id) => {
    // ID is full path
    try {
      await deletePath(id);
      await get().refresh();
    } catch (e) {
      console.error('Failed to delete', e);
    }
  },

  updateFileContent: async (id, content) => {
    // Persist to disk
    try {
      await writeFile(id, content);
      // Also update memory
      set({ files: updateFileContentInTree(get().files, id, content) });
    } catch (e) {
      console.error('Failed to save file', e);
    }
  },

  toggleFolder: (id) => {
    set({ files: toggleFolderInTree(get().files, id) });
  },

  renameFile: async (id, newName) => {
    // id is old path
    // we need to construct new path
    // Split id by separator... path API is needed
    // Assuming simple rename in same dir
    // This is a bit tricky without 'dirname' helper in tauriUtils or path-browserify
    // Let's assume we can get parent path from the tree structure or string manipulation

    // Basic string manip for now (Unix/Win compat might be an issue but Tauri pathJoin handles some)
    // Actually standard JS string replacement for filename at end of path:
    // Last slash or backslash

    // Helper to get dir from path
    const getDir = (p: string) => p.substring(0, Math.max(p.lastIndexOf('/'), p.lastIndexOf('\\')));
    const parentDir = getDir(id);

    const newPath = await pathJoin(parentDir, newName);

    try {
      await renamePath(id, newPath);
      await get().refresh();
    } catch (e) {
      console.error('Failed to rename', e);
    }
  },

  moveFile: async (_id, _newParentId, _index) => {
    // Skipping complicated move (drag & drop) for now to ensure stability 
    // as it involves moving files on disk and potentially recursively
    console.warn('Move not fully implemented for FS');
  },

  getFileContentByName: (name: string) => {
    // This is used for simple lookups, might fail if multiple files have same name
    // This legacy method assumes unique names or flat list maybe.
    const file = findFileByName(get().files, name);
    return file?.content || null;
  },

  loadContent: async (id: string) => {
    try {
      const content = await readFile(id);
      set({ files: updateFileContentInTree(get().files, id, content) });
    } catch (e) {
      console.error('Failed to load content', e);
    }
  },
}));

