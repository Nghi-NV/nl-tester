import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { TestResult } from '../types';
import { generateRunId } from '../utils/idGenerator';
import { useExecutionStateStore } from './executionStateStore';

interface ExecutionStore {
  isRunning: boolean;
  abortController: AbortController | null;
  runningNodeIds: string[];
  currentRunId: string | null;
  results: TestResult[];

  // Actions
  startRun: () => AbortSignal;
  stopRun: () => void;
  setNodeRunning: (id: string, isRunning: boolean) => void;
  addResult: (result: TestResult) => void;
  upsertResult: (result: TestResult) => void;
  clearResults: () => void;
}

export const useExecutionStore = create<ExecutionStore>()(
  persist(
    (set, get) => ({
      isRunning: false,
      abortController: null,
      runningNodeIds: [],
      currentRunId: null,
      results: [],

      startRun: () => {
        const ac = new AbortController();
        // Clear all execution states when starting a new run
        useExecutionStateStore.getState().clearAllStates();
        set({
          isRunning: true,
          abortController: ac,
          currentRunId: generateRunId(),
        });
        return ac.signal;
      },

      stopRun: () => {
        const { abortController } = get();
        if (abortController) {
          abortController.abort();
        }
        set({
          isRunning: false,
          abortController: null,
          runningNodeIds: [],
        });
      },

      setNodeRunning: (id, isRunning) => set(state => {
        if (isRunning) {
          return { runningNodeIds: [...state.runningNodeIds, id] };
        } else {
          return { runningNodeIds: state.runningNodeIds.filter(nid => nid !== id) };
        }
      }),

      addResult: (result) => set(state => ({ results: [result, ...state.results] })),

      upsertResult: (result) => set(state => {
        const index = state.results.findIndex(r => r.id === result.id);
        if (index >= 0) {
          const newResults = [...state.results];
          newResults[index] = result;
          return { results: newResults };
        } else {
          return { results: [result, ...state.results] };
        }
      }),

      clearResults: () => set({ results: [] }),
    }),
    {
      name: 'nexus-execution-store',
      partialize: (state) => ({ results: state.results }), // Only persist results, not running state
    }
  )
);
