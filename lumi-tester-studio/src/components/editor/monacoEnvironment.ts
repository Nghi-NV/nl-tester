import EditorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker';

(globalThis as typeof globalThis & {
  MonacoEnvironment: { getWorker: (_moduleId: string, _label: string) => Worker };
}).MonacoEnvironment = {
  getWorker: () => new EditorWorker(),
};
