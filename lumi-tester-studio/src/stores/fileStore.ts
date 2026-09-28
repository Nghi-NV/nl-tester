import { create } from 'zustand';
import { useEditorStore } from './editorStore';
import { FileNode } from '../types';
import {
  findFileById,
  findFileByName,
  toggleFolderInTree,
  updateFileContentInTree,
  updateNodeInTree,
} from '../utils/treeUtils';
import { FILE_CONFIG } from '../constants';
import {
  createDir,
  createFile,
  deletePath,
  openWorkspace,
  readDir,
  readFile,
  renamePath,
  resolveWorkspaceFileReference,
  writeFile,
  pathJoin,
  pathDirname,
} from '../utils/tauriUtils';

export interface FileValidation {
  state: 'checking' | 'valid' | 'invalid' | 'error';
  message?: string;
}

interface FileStore {
  files: FileNode[];
  projectRoot: string | null;
  isLoading: boolean;
  showHiddenFiles: boolean;
  dirtyFileIds: string[];
  fileValidation: Record<string, FileValidation>;
  loadProject: (path: string) => Promise<void>;
  addFile: (parentId: string | null, type: 'file' | 'folder', name: string) => Promise<void>;
  deleteFile: (id: string) => Promise<void>;
  renameFile: (id: string, newName: string) => Promise<void>;
  moveFile: (id: string, newParentId: string | null, index: number) => Promise<void>;
  updateFileContent: (id: string, content: string) => void;
  setFileValidation: (id: string, validation?: FileValidation) => void;
  saveFile: (id: string) => Promise<void>;
  saveAllFiles: () => Promise<void>;
  discardFileChanges: (id: string) => Promise<void>;
  toggleFolder: (id: string) => Promise<void>;
  setShowHiddenFiles: (show: boolean) => Promise<void>;
  getFileContentByName: (name: string) => string | null;
  loadContent: (id: string) => Promise<void>;
  loadDescendantYamlFiles: (id: string) => Promise<FileNode[]>;
  openReferencedFile: (sourcePath: string, reference: string) => Promise<string>;
  openSearchResult: (relativePath: string, lineNumber: number, column: number) => Promise<void>;
  refresh: () => Promise<void>;
}

const collectOpenDirectories = (nodes: FileNode[]): Set<string> => {
  const open = new Set<string>();
  for (const node of nodes) {
    if (node.type === 'folder' && node.isOpen) open.add(node.id);
    if (node.children) {
      for (const path of collectOpenDirectories(node.children)) open.add(path);
    }
  }
  return open;
};

const collectLoadedDirectories = (nodes: FileNode[]): Set<string> => {
  const loaded = new Set<string>();
  for (const node of nodes) {
    if (node.type === 'folder' && node.children !== undefined) loaded.add(node.id);
    if (node.children) {
      for (const path of collectLoadedDirectories(node.children)) loaded.add(path);
    }
  }
  return loaded;
};

const buildFileTree = async (
  path: string,
  showHidden: boolean,
  loadedDirectories: Set<string> = new Set(),
  openDirectories: Set<string> = new Set(),
): Promise<FileNode[]> => {
  const entries = await readDir(path, showHidden);
  return Promise.all(entries.map(async entry => {
    const isOpen = entry.isDirectory && openDirectories.has(entry.path);
    return {
      id: entry.path,
      name: entry.name,
      type: entry.isDirectory ? 'folder' as const : 'file' as const,
      children: entry.isDirectory && loadedDirectories.has(entry.path)
        ? await buildFileTree(entry.path, showHidden, loadedDirectories, openDirectories)
        : undefined,
      content: undefined,
      isOpen,
    };
  }));
};

const preserveDirtyContent = (fresh: FileNode[], previous: FileNode[], dirty: string[]): FileNode[] => {
  const previousById = new Map<string, FileNode>();
  const index = (nodes: FileNode[]) => nodes.forEach(node => {
    previousById.set(node.id, node);
    if (node.children) index(node.children);
  });
  index(previous);

  const dirtySet = new Set(dirty);
  const merge = (nodes: FileNode[]): FileNode[] => nodes.map(node => {
    const old = previousById.get(node.id);
    return {
      ...node,
      ...(old && dirtySet.has(node.id) ? { content: old.content } : {}),
      children: node.children ? merge(node.children) : undefined,
    };
  });
  return merge(fresh);
};

const removeDirtyPath = (dirty: string[], id: string) => dirty.filter(path => path !== id);

const insertReferencedFile = (
  nodes: FileNode[],
  ids: string[],
  names: string[],
  depth: number,
  content: string,
  preserveContent: boolean,
): FileNode[] => {
  const id = ids[depth];
  const name = names[depth];
  const isFile = depth === ids.length - 1;
  const index = nodes.findIndex(node => node.id === id);
  const existing = index >= 0 ? nodes[index] : undefined;
  const node: FileNode = isFile
    ? {
      id,
      name,
      type: 'file',
      content: preserveContent ? existing?.content ?? content : content,
    }
    : {
      id,
      name,
      type: 'folder',
      isOpen: true,
      children: insertReferencedFile(existing?.children ?? [], ids, names, depth + 1, content, preserveContent),
    };
  if (index < 0) return [...nodes, node];
  return nodes.map((entry, entryIndex) => entryIndex === index ? node : entry);
};

export const useFileStore = create<FileStore>((set, get) => ({
  files: [],
  projectRoot: null,
  isLoading: false,
  showHiddenFiles: true,
  dirtyFileIds: [],
  fileValidation: {},

  loadProject: async (path: string) => {
    set({ isLoading: true });
    try {
      const workspace = await openWorkspace(path);
      const files = await buildFileTree(workspace.path, get().showHiddenFiles);
      useEditorStore.getState().closeAllFiles();
      set({ files, projectRoot: workspace.path, dirtyFileIds: [] });
      localStorage.setItem('lumi_project_root', workspace.path);
    } catch (error) {
      console.error('Failed to load project', error);
      throw error;
    } finally {
      set({ isLoading: false });
    }
  },

  refresh: async () => {
    const { projectRoot, files, dirtyFileIds, showHiddenFiles } = get();
    if (!projectRoot) return;
    const openDirectories = collectOpenDirectories(files);
    const refreshed = await buildFileTree(
      projectRoot,
      showHiddenFiles,
      collectLoadedDirectories(files),
      openDirectories,
    );
    set({ files: preserveDirtyContent(refreshed, files, dirtyFileIds) });
  },

  addFile: async (parentId, type, name) => {
    const root = get().projectRoot;
    if (!root) return;
    const parent = parentId || root;
    if (type === 'folder') {
      await createDir(parent, name);
    } else {
      await createFile(parent, name, FILE_CONFIG.DEFAULT_FILE_CONTENT || '');
    }
    await get().refresh();
  },

  deleteFile: async id => {
    await deletePath(id);
    set(state => ({ dirtyFileIds: state.dirtyFileIds.filter(path =>
      path !== id && !path.startsWith(`${id}/`) && !path.startsWith(`${id}\\`),
    ) }));
    await get().refresh();
  },

  renameFile: async (id, newName) => {
    if (get().dirtyFileIds.some(path => path === id || path.startsWith(`${id}/`) || path.startsWith(`${id}\\`))) {
      await get().saveAllFiles();
    }
    const parent = await pathDirname(id);
    const newPath = await pathJoin(parent, newName);
    await renamePath(id, newPath);
    useEditorStore.getState().replaceFilePath(id, newPath);
    await get().refresh();
  },

  moveFile: async (id, newParentId, _index) => {
    const source = findFileById(get().files, id);
    const destination = newParentId || get().projectRoot;
    if (!source || !destination) return;
    if (get().dirtyFileIds.some(path => path === id || path.startsWith(`${id}/`) || path.startsWith(`${id}\\`))) {
      await get().saveAllFiles();
    }
    const newPath = await pathJoin(destination, source.name);
    await renamePath(id, newPath);
    useEditorStore.getState().replaceFilePath(id, newPath);
    await get().refresh();
  },

  updateFileContent: (id, content) => {
    set(state => ({
      files: updateFileContentInTree(state.files, id, content),
      dirtyFileIds: state.dirtyFileIds.includes(id) ? state.dirtyFileIds : [...state.dirtyFileIds, id],
    }));
  },

  setFileValidation: (id, validation) => set(state => {
    const fileValidation = { ...state.fileValidation };
    if (validation) fileValidation[id] = validation;
    else delete fileValidation[id];
    return { fileValidation };
  }),

  saveFile: async id => {
    const node = findFileById(get().files, id);
    if (!node || node.type !== 'file' || node.content === undefined) return;
    const content = node.content;
    await writeFile(id, content);
    set(state => {
      const latestNode = findFileById(state.files, id);
      return latestNode?.content === content
        ? { dirtyFileIds: removeDirtyPath(state.dirtyFileIds, id) }
        : {};
    });
  },

  saveAllFiles: async () => {
    const ids = [...get().dirtyFileIds];
    for (const id of ids) await get().saveFile(id);
  },

  discardFileChanges: async id => {
    const content = await readFile(id);
    set(state => ({
      files: updateFileContentInTree(state.files, id, content),
      dirtyFileIds: removeDirtyPath(state.dirtyFileIds, id),
    }));
  },

  toggleFolder: async id => {
    const node = findFileById(get().files, id);
    if (!node || node.type !== 'folder') return;
    if (node.isOpen || node.children !== undefined) {
      set(state => ({ files: toggleFolderInTree(state.files, id) }));
      return;
    }

    const children = await buildFileTree(id, get().showHiddenFiles);
    set(state => ({
      files: updateNodeInTree(state.files, id, current => ({ ...current, children, isOpen: true })),
    }));
  },

  setShowHiddenFiles: async show => {
    set({ showHiddenFiles: show });
    await get().refresh();
  },

  getFileContentByName: name => findFileByName(get().files, name)?.content ?? null,

  loadContent: async id => {
    try {
      const content = await readFile(id);
      set(state => ({ files: updateFileContentInTree(state.files, id, content) }));
    } catch (error) {
      console.error('Failed to load file', error);
      throw error;
    }
  },

  loadDescendantYamlFiles: async id => {
    const showHidden = get().showHiddenFiles;
    const collect = async (path: string): Promise<FileNode[]> => {
      const node = findFileById(get().files, path);
      if (node?.type === 'file') {
        if (!/\.ya?ml$/i.test(node.name)) return [];
        return [{ ...node, content: node.content ?? await readFile(path) }];
      }

      const entries = await readDir(path, showHidden);
      const files = await Promise.all(entries.map(async entry => {
        if (entry.isDirectory) return collect(entry.path);
        if (!/\.ya?ml$/i.test(entry.name)) return [];

        const cached = findFileById(get().files, entry.path);
        const content = cached?.content ?? await readFile(entry.path);
        return [{
          id: entry.path,
          name: entry.name,
          type: 'file' as const,
          content,
        }];
      }));
      return files.flat();
    };

    return collect(id);
  },

  openReferencedFile: async (sourcePath, reference) => {
    const root = get().projectRoot;
    if (!root) throw new Error('Open a workspace first.');
    const resolved = await resolveWorkspaceFileReference(sourcePath, reference);
    const content = await readFile(resolved.path);
    const names = resolved.relativePath.split(/[\\/]/).filter(Boolean);
    const ids: string[] = [];
    let path = root;
    for (const name of names) {
      path = await pathJoin(path, name);
      ids.push(path);
    }
    if (ids.length === 0) throw new Error('The referenced file is outside the open workspace.');
    const preserveContent = get().dirtyFileIds.includes(resolved.path);
    set(state => ({
      files: insertReferencedFile(state.files, ids, names, 0, content, preserveContent),
    }));
    return resolved.path;
  },

  openSearchResult: async (relativePath, lineNumber, column) => {
    const root = get().projectRoot;
    if (!root) throw new Error('Open a workspace first.');
    const names = relativePath.split(/[\\/]/).filter(Boolean);
    if (names.length === 0 || names.some(name => name === '.' || name === '..')) {
      throw new Error('The search result path is invalid.');
    }

    const ids: string[] = [];
    let path = root;
    for (const name of names) {
      path = await pathJoin(path, name);
      ids.push(path);
    }
    const filePath = ids[ids.length - 1];
    const content = await readFile(filePath);
    const preserveContent = get().dirtyFileIds.includes(filePath);
    set(state => ({
      files: insertReferencedFile(state.files, ids, names, 0, content, preserveContent),
    }));
    useEditorStore.getState().openFileAt(filePath, lineNumber, column);
  },
}));
