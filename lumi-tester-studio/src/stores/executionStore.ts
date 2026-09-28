import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { TestResult } from '../types';
import { generateRunId } from '../utils/idGenerator';
import { useExecutionStateStore } from './executionStateStore';

export interface QueuedRun {
  id: string;
  label: string;
}

export interface RunContext {
  signal: AbortSignal;
  runId: string;
}

type RunTask = QueuedRun & {
  execute: (context: RunContext) => Promise<void>;
  resolve: () => void;
  reject: (error: unknown) => void;
};

interface ExecutionStore {
  isRunning: boolean;
  abortController: AbortController | null;
  runningNodeIds: string[];
  currentRunId: string | null;
  currentRunLabel: string | null;
  queuedRuns: QueuedRun[];
  lastRunRequest: QueuedRun | null;
  runRequestVersion: number;
  results: TestResult[];

  queueRun: (label: string, execute: (context: RunContext) => Promise<void>) => Promise<void>;
  stopRun: () => void;
  setNodeRunning: (id: string, isRunning: boolean) => void;
  addResult: (result: TestResult) => void;
  upsertResult: (result: TestResult) => void;
  clearResults: () => void;
}

export const useExecutionStore = create<ExecutionStore>()(
  persist(
    (set, get) => {
      const pendingRuns: RunTask[] = [];

      const runNext = () => {
        if (get().isRunning) return;
        const task = pendingRuns.shift();
        if (!task) return;

        const controller = new AbortController();
        useExecutionStateStore.getState().clearAllStates();
        set(state => ({
          isRunning: true,
          abortController: controller,
          currentRunId: task.id,
          currentRunLabel: task.label,
          queuedRuns: state.queuedRuns.filter(run => run.id !== task.id),
        }));

        void (async () => {
          try {
            await task.execute({ signal: controller.signal, runId: task.id });
            task.resolve();
          } catch (error) {
            task.reject(error);
          } finally {
            if (get().currentRunId === task.id) {
              set({
                isRunning: false,
                abortController: null,
                currentRunId: null,
                currentRunLabel: null,
                runningNodeIds: [],
              });
            }
            runNext();
          }
        })();
      };

      return {
        isRunning: false,
        abortController: null,
        runningNodeIds: [],
        currentRunId: null,
        currentRunLabel: null,
        queuedRuns: [],
        lastRunRequest: null,
        runRequestVersion: 0,
        results: [],

        queueRun: (label, execute) => new Promise<void>((resolve, reject) => {
          const task: RunTask = { id: generateRunId(), label, execute, resolve, reject };
          pendingRuns.push(task);
          set(state => ({
            queuedRuns: [...state.queuedRuns, { id: task.id, label }],
            lastRunRequest: { id: task.id, label },
            runRequestVersion: state.runRequestVersion + 1,
          }));
          runNext();
        }),

        stopRun: () => {
          get().abortController?.abort();
        },

        setNodeRunning: (id, isRunning) => set(state => {
          if (isRunning) {
            return { runningNodeIds: state.runningNodeIds.includes(id) ? state.runningNodeIds : [...state.runningNodeIds, id] };
          }
          return { runningNodeIds: state.runningNodeIds.filter(nodeId => nodeId !== id) };
        }),

        addResult: result => set(state => ({ results: [result, ...state.results] })),

        upsertResult: result => set(state => {
          const index = state.results.findIndex(existing => existing.id === result.id);
          if (index >= 0) {
            const results = [...state.results];
            results[index] = result;
            return { results };
          }
          return { results: [result, ...state.results] };
        }),

        clearResults: () => set({ results: [] }),
      };
    },
    {
      name: 'nexus-execution-store',
      partialize: state => ({ results: state.results }),
    },
  ),
);
