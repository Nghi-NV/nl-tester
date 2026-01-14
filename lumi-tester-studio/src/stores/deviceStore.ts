import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { invoke } from '@tauri-apps/api/core';

export interface Device {
  id: string; // Serial
  name: string; // Model or Serial
  platform: 'android' | 'ios' | 'web';
}

interface DeviceStore {
  devices: Device[];
  allDevices: Device[]; // All devices from all platforms
  selectedDevice: string | null; // Serial
  selectedPlatform: 'android' | 'ios' | 'web';
  isLoading: boolean;

  // Actions
  refreshDevices: () => Promise<void>;
  refreshAllDevices: () => Promise<void>; // Load all devices from all platforms
  setSelectedDevice: (deviceId: string | null) => void;
  setSelectedPlatform: (platform: 'android' | 'ios' | 'web') => void;
}

export const useDeviceStore = create<DeviceStore>()(
  persist(
    (set, get) => ({
      devices: [],
      allDevices: [],
      selectedDevice: null,
      selectedPlatform: 'android',
      isLoading: false,

      refreshDevices: async () => {
        set({ isLoading: true });
        try {
          const platform = get().selectedPlatform;
          if (platform === 'android' || platform === 'ios') {
            const deviceInfos = await invoke<Array<{ id: string; name: string }>>('list_devices', { platform });
            const devices: Device[] = deviceInfos.map(device => ({
              id: device.id,
              name: device.name,
              platform: platform
            }));
            set({ devices });

            // Auto-select if none selected or not in list
            const current = get().selectedDevice;
            if (devices.length > 0 && (!current || !devices.find(d => d.id === current))) {
              set({ selectedDevice: devices[0].id });
            }
          } else {
            set({ devices: [] });
          }
        } catch (error) {
          console.error('Failed to list devices', error);
          set({ devices: [] });
        } finally {
          set({ isLoading: false });
        }
      },

      refreshAllDevices: async () => {
        set({ isLoading: true });
        try {
          const allDevicesList: Device[] = [];
          
          // Load Android devices
          try {
            const androidDevices = await invoke<Array<{ id: string; name: string }>>('list_devices', { platform: 'android' });
            androidDevices.forEach(device => {
              allDevicesList.push({
                id: device.id,
                name: device.name,
                platform: 'android'
              });
            });
          } catch (error) {
            console.error('Failed to load Android devices', error);
          }

          // Load iOS devices
          try {
            const iosDevices = await invoke<Array<{ id: string; name: string }>>('list_devices', { platform: 'ios' });
            iosDevices.forEach(device => {
              allDevicesList.push({
                id: device.id,
                name: device.name,
                platform: 'ios'
              });
            });
          } catch (error) {
            console.error('Failed to load iOS devices', error);
          }

          set({ allDevices: allDevicesList });
        } catch (error) {
          console.error('Failed to refresh all devices', error);
          set({ allDevices: [] });
        } finally {
          set({ isLoading: false });
        }
      },

      setSelectedDevice: (deviceId) => set({ selectedDevice: deviceId }),

      setSelectedPlatform: (platform) => {
        set({ selectedPlatform: platform, selectedDevice: null });
        get().refreshDevices();
      }
    }),
    {
      name: 'lumi-device-store',
      partialize: (state) => ({
        selectedPlatform: state.selectedPlatform,
        selectedDevice: state.selectedDevice
      }),
    }
  )
);
