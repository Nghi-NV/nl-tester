import { create } from 'zustand';
import { AiMessage, AiConfig } from '../types';
import { generateId } from '../utils/idGenerator';
import { AI_CONFIG } from '../constants';

interface AiStore {
  isAiOpen: boolean;
  aiMessages: AiMessage[];
  aiConfig: AiConfig;
  isAiLoading: boolean;

  // Actions
  toggleAi: () => void;
  addAiMessage: (message: Omit<AiMessage, 'id' | 'timestamp'>) => void;
  setAiConfig: (config: Partial<AiConfig>) => void;
  setAiLoading: (loading: boolean) => void;
  clearAiChat: () => void;
}

export const useAiStore = create<AiStore>((set) => ({
  isAiOpen: false,
  aiMessages: [{
    id: 'init',
    role: 'model',
    content: AI_CONFIG.INITIAL_MESSAGE,
    timestamp: Date.now()
  }],
  aiConfig: {
    apiKey: '',
    model: AI_CONFIG.DEFAULT_MODEL
  },
  isAiLoading: false,

  toggleAi: () => set(state => ({ isAiOpen: !state.isAiOpen })),

  addAiMessage: (msg) => set(state => ({
    aiMessages: [...state.aiMessages, { ...msg, id: generateId(), timestamp: Date.now() }]
  })),

  setAiConfig: (config) => set(state => ({ aiConfig: { ...state.aiConfig, ...config } })),

  setAiLoading: (loading) => set({ isAiLoading: loading }),

  clearAiChat: () => set({
    aiMessages: [{
      id: 'init',
      role: 'model',
      content: AI_CONFIG.CLEARED_MESSAGE,
      timestamp: Date.now()
    }]
  }),
}));
