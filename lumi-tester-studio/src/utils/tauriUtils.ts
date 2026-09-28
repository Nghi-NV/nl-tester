import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { basename, dirname, homeDir, join } from '@tauri-apps/api/path';

export interface WorkspaceEntry {
  name: string;
  path: string;
  isDirectory: boolean;
}

export interface WorkspaceFileReference {
  path: string;
  relativePath: string;
  name: string;
}

export interface WorkspaceSearchMatch {
  relativePath: string;
  lineNumber: number;
  column: number;
  line: string;
}

export interface WorkspaceSearchResponse {
  matches: WorkspaceSearchMatch[];
  filesScanned: number;
  truncated: boolean;
}

export const isTauri = () => '__TAURI_INTERNALS__' in window;

// File System Wrappers
export const readDir = async (path: string, showHidden = true): Promise<WorkspaceEntry[]> => {
  if (!isTauri()) {
    console.warn('Tauri not detected. Mocking readDir.');
    return [];
  }
  return invoke<WorkspaceEntry[]>('read_workspace_dir', { path, showHidden });
};

export const readFile = async (path: string) => {
  if (!isTauri()) {
    console.warn('Tauri not detected. Mocking readFile.');
    return '';
  }
  return invoke<string>('read_workspace_file', { path });
};

export const writeFile = async (path: string, content: string) => {
  if (!isTauri()) {
    console.warn('Tauri not detected. Mocking writeFile.');
    return;
  }
  await invoke('write_workspace_file', { path, content });
};

export const deletePath = async (path: string) => {
  if (!isTauri()) {
    console.warn('Tauri not detected. Mocking deletePath.');
    return;
  }
  await invoke('remove_workspace_entry', { path });
}

export const renamePath = async (oldPath: string, newPath: string) => {
  if (!isTauri()) {
    console.warn('Tauri not detected. Mocking renamePath.');
    return;
  }
  await invoke('move_workspace_entry', {
    path: oldPath,
    destination: await dirname(newPath),
    name: await basename(newPath),
  });
}

export const createDir = async (parent: string, name: string) => {
  if (!isTauri()) {
    console.warn('Tauri not detected. Mocking createDir.');
    return;
  }
  await invoke('create_workspace_dir', { parent, name });
}

export const createFile = async (parent: string, name: string, content: string) => {
  if (!isTauri()) {
    console.warn('Tauri not detected. Mocking createFile.');
    return;
  }
  await invoke('create_workspace_file', { parent, name, content });
}

export const openWorkspace = async (path: string) => {
  if (!isTauri()) return { path, name: path.split(/[\\/]/).filter(Boolean).pop() || 'Workspace' };
  return invoke<{ path: string; name: string }>('open_workspace', { path });
}

export const resolveWorkspaceFileReference = async (sourcePath: string, reference: string) => {
  if (!isTauri()) throw new Error('Open a Lumi IDE workspace before opening referenced files.');
  return invoke<WorkspaceFileReference>('resolve_workspace_file_reference', { sourcePath, reference });
}

export const searchWorkspaceText = async (workspacePath: string, query: string) => {
  if (!isTauri()) throw new Error('Workspace search is available in the Lumi IDE desktop app.');
  return invoke<WorkspaceSearchResponse>('search_workspace_text', { workspacePath, query });
}

export const openDialog = async (options: any) => {
  if (!isTauri()) {
    console.warn('Tauri not detected. Mocking openDialog.');
    return null;
  }
  return await open(options);
}


export const getHomeDir = async () => {
  if (!isTauri()) return '/mock/home';
  return await homeDir();
}

export const pathJoin = async (...args: string[]) => {
  if (!isTauri()) return args.join('/');
  return await join(...args);
}

export const pathDirname = async (path: string) => {
  if (!isTauri()) return path.replace(/[\\/][^\\/]*$/, '') || '/';
  return await dirname(path);
}
