import { readDir as tauriReadDir, readTextFile as tauriReadTextFile, writeTextFile as tauriWriteTextFile, remove as tauriRemove, rename as tauriRename, mkdir as tauriMkdir } from '@tauri-apps/plugin-fs';
import { open } from '@tauri-apps/plugin-dialog';
import { Command } from '@tauri-apps/plugin-shell';
import { homeDir, join } from '@tauri-apps/api/path';

export const isTauri = () => '__TAURI_INTERNALS__' in window;

// File System Wrappers
export const readDir = async (path: string) => {
  if (!isTauri()) {
    console.warn('Tauri not detected. Mocking readDir.');
    return [];
  }
  return await tauriReadDir(path);
};

export const readFile = async (path: string) => {
  if (!isTauri()) {
    console.warn('Tauri not detected. Mocking readFile.');
    return '';
  }
  return await tauriReadTextFile(path);
};

export const writeFile = async (path: string, content: string) => {
  if (!isTauri()) {
    console.warn('Tauri not detected. Mocking writeFile.');
    return;
  }
  await tauriWriteTextFile(path, content);
};

export const deletePath = async (path: string) => {
  if (!isTauri()) {
    console.warn('Tauri not detected. Mocking deletePath.');
    return;
  }
  await tauriRemove(path, { recursive: true });
}

export const renamePath = async (oldPath: string, newPath: string) => {
  if (!isTauri()) {
    console.warn('Tauri not detected. Mocking renamePath.');
    return;
  }
  await tauriRename(oldPath, newPath);
}

export const createDir = async (path: string) => {
  if (!isTauri()) {
    console.warn('Tauri not detected. Mocking createDir.');
    return;
  }
  await tauriMkdir(path, { recursive: true });
}

export const openDialog = async (options: any) => {
  if (!isTauri()) {
    console.warn('Tauri not detected. Mocking openDialog.');
    return null;
  }
  return await open(options);
}


// Shell Wrappers
export const runCommand = async (command: string, args: string[]) => {
  if (!isTauri()) {
    console.warn('Tauri not detected. Mocking runCommand.');
    return { code: 0, stdout: '', stderr: '' };
  }
  const cmd = Command.create(command, args);
  return await cmd.execute();
};

export type SpawnCallbacks = {
  onOutput: (data: string) => void;
  onError: (data: string) => void;
  onClose: (code: number) => void;
};

export const spawnCommand = async (command: string, args: string[], callbacks: SpawnCallbacks) => {
  if (!isTauri()) {
    console.warn('Tauri not detected. Mocking spawnCommand.');
    setTimeout(() => {
      callbacks.onOutput('Mock output: Command started\n');
      callbacks.onOutput('Mock output: Executing step...\n');
      callbacks.onClose(0);
    }, 1000);
    return { kill: async () => { } };
  }

  const cmd = Command.create(command, args);

  cmd.on('close', (data: any) => {
    callbacks.onClose(data.code);
  });

  cmd.on('error', (error: any) => {
    callbacks.onError(String(error));
  });

  cmd.stdout.on('data', (line: any) => {
    callbacks.onOutput(line);
  });

  cmd.stderr.on('data', (line: any) => {
    callbacks.onError(line);
  });

  const child = await cmd.spawn();

  return {
    kill: async () => {
      await child.kill();
    }
  };
};

export const getHomeDir = async () => {
  if (!isTauri()) return '/mock/home';
  return await homeDir();
}

export const pathJoin = async (...args: string[]) => {
  if (!isTauri()) return args.join('/');
  return await join(...args);
}
