import { create } from 'zustand';
import { EnvVar } from '../types';

interface EnvStore {
  envVars: EnvVar[];
  setEnvVars: (vars: EnvVar[]) => void;
}

export const useEnvStore = create<EnvStore>((set) => ({
  envVars: [{ key: 'user_name', value: 'Neo', enabled: true }],
  setEnvVars: (vars) => set({ envVars: vars }),
}));
