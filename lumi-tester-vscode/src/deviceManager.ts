import * as vscode from 'vscode';
import * as cp from 'child_process';

export interface Device {
  id: string;
  name: string;
  platform: 'android' | 'ios' | 'web';
  state: string;
  type: 'physical' | 'simulator' | 'emulator' | 'browser';
}

export class DeviceManager {
  private static instance: DeviceManager;
  private cachedDevices: Device[] = [];
  private selectedDevice: Device | null = null;
  private statusBarItem: vscode.StatusBarItem;
  private lastRefresh: number = 0;
  private cacheTimeout = 10000;
  private initialized = false;

  private constructor() {
    this.statusBarItem = vscode.window.createStatusBarItem(
      vscode.StatusBarAlignment.Right,
      50
    );
    this.statusBarItem.command = 'lumi-tester.selectDevice';
    this.updateStatusBar();

    vscode.window.onDidChangeActiveTextEditor((editor) => {
      this.updateStatusBarVisibility(editor);
    });

    this.updateStatusBarVisibility(vscode.window.activeTextEditor);

    // Auto-select default device on startup
    this.autoSelectDefaultDevice();
  }

  private async autoSelectDefaultDevice(): Promise<void> {
    if (this.initialized) return;
    this.initialized = true;

    const devices = await this.refreshDevices(true);
    const availableDevices = devices.filter(d => {
      const state = d.state.toLowerCase();
      return state === 'device' || state === 'available';
    });

    if (availableDevices.length > 0) {
      // Priority: physical devices first, then emulators/simulators, then web
      const physicalDevices = availableDevices.filter(d => d.type === 'physical');
      const emulatorDevices = availableDevices.filter(d => d.type === 'emulator' || d.type === 'simulator');
      const webDevices = availableDevices.filter(d => d.type === 'browser');

      if (physicalDevices.length > 0) {
        this.setSelectedDevice(physicalDevices[0]);
      } else if (emulatorDevices.length > 0) {
        this.setSelectedDevice(emulatorDevices[0]);
      } else if (webDevices.length > 0) {
        this.setSelectedDevice(webDevices[0]);
      }
    }
  }

  private updateStatusBarVisibility(editor: vscode.TextEditor | undefined): void {
    if (editor && editor.document.languageId === 'yaml') {
      this.statusBarItem.show();
    } else {
      this.statusBarItem.hide();
    }
  }

  public static getInstance(): DeviceManager {
    if (!DeviceManager.instance) {
      DeviceManager.instance = new DeviceManager();
    }
    return DeviceManager.instance;
  }

  public getSelectedDevice(): Device | null {
    return this.selectedDevice;
  }

  public setSelectedDevice(device: Device | null): void {
    this.selectedDevice = device;
    this.updateStatusBar();
  }

  private updateStatusBar(): void {
    if (this.selectedDevice) {
      this.statusBarItem.text = `${this.selectedDevice.name} (${this.selectedDevice.platform})`;
    } else {
      this.statusBarItem.text = 'No destination selected';
    }
  }

  public async refreshDevices(forceRefresh: boolean = false): Promise<Device[]> {
    const now = Date.now();
    if (!forceRefresh && now - this.lastRefresh < this.cacheTimeout && this.cachedDevices.length > 0) {
      return this.cachedDevices;
    }

    const devices: Device[] = [];

    const [androidDevices, iosDevices] = await Promise.all([
      this.fetchAndroidDevices(),
      this.fetchIOSDevices()
    ]);

    devices.push(...androidDevices);
    devices.push(...iosDevices);

    // Add Chrome browser
    devices.push({
      id: 'chrome',
      name: 'Chrome',
      platform: 'web',
      state: 'Available',
      type: 'browser'
    });

    this.cachedDevices = devices;
    this.lastRefresh = now;
    return devices;
  }

  private fetchAndroidDevices(): Promise<Device[]> {
    return new Promise((resolve) => {
      cp.exec('adb devices -l', { timeout: 10000 }, (error, stdout) => {
        if (error) {
          resolve([]);
          return;
        }

        const devices: Device[] = [];
        const lines = stdout.split('\n').slice(1);

        for (const line of lines) {
          if (!line.trim()) continue;

          const match = line.match(/^(\S+)\s+(device|offline|unauthorized).*?model:(\S+)/);
          if (match) {
            const id = match[1];
            const state = match[2];
            const model = match[3].replace(/_/g, ' ');
            const isEmulator = id.startsWith('emulator-');

            devices.push({
              id: id,
              name: model,
              platform: 'android',
              state: state,
              type: isEmulator ? 'emulator' : 'physical'
            });
          } else {
            const simpleMatch = line.match(/^(\S+)\s+(device|offline)/);
            if (simpleMatch) {
              const id = simpleMatch[1];
              const state = simpleMatch[2];
              devices.push({
                id: id,
                name: id,
                platform: 'android',
                state: state,
                type: id.startsWith('emulator-') ? 'emulator' : 'physical'
              });
            }
          }
        }

        resolve(devices);
      });
    });
  }

  private fetchIOSDevices(): Promise<Device[]> {
    return new Promise((resolve) => {
      cp.exec('xcrun xctrace list devices 2>&1', { timeout: 10000 }, (error, stdout) => {
        if (error) {
          resolve([]);
          return;
        }

        const devices: Device[] = [];
        const lines = stdout.split('\n');
        let inDevicesSection = false;
        let inOfflineSection = false;
        let inSimulatorsSection = false;

        for (const line of lines) {
          if (line.includes('== Devices ==')) {
            inDevicesSection = true;
            inOfflineSection = false;
            inSimulatorsSection = false;
            continue;
          }
          if (line.includes('== Devices Offline ==')) {
            inDevicesSection = false;
            inOfflineSection = true;
            inSimulatorsSection = false;
            continue;
          }
          if (line.includes('== Simulators ==')) {
            inDevicesSection = false;
            inOfflineSection = false;
            inSimulatorsSection = true;
            continue;
          }

          const match = line.match(/^(.+?)\s+\(([^)]+)\)\s+\(([A-F0-9-]+)\)/);
          if (match) {
            const name = match[1].trim();
            const udid = match[3].trim();

            // Skip non-mobile devices: Mac, Watch, Apple TV, Apple Vision
            const nameLower = name.toLowerCase();
            if (nameLower.includes('mac') ||
              nameLower.includes('watch') ||
              nameLower.includes('apple tv') ||
              nameLower.includes('vision')) continue;

            let type: 'physical' | 'simulator' | 'emulator' | 'browser' = 'physical';
            let state = 'Available';

            if (inSimulatorsSection) {
              type = 'simulator';
              state = 'Shutdown';
            } else if (inOfflineSection) {
              state = 'Offline';
            }

            // Simulators have UUID format (36 chars with dashes)
            if (udid.length === 36 && udid.split('-').length === 5) {
              type = 'simulator';
            }

            devices.push({
              id: udid,
              name: name,
              platform: 'ios',
              state: state,
              type: type
            });
          }
        }

        resolve(devices);
      });
    });
  }

  private getDeviceIcon(device: Device): string {
    if (device.platform === 'web') {
      return '$(globe)';
    } else if (device.platform === 'ios') {
      return '$(device-mobile)';
    } else {
      // Android
      return '$(vm)';
    }
  }

  public async showDevicePicker(): Promise<Device | undefined> {
    // Use cached devices first for fast display, then refresh in background
    let devices = this.cachedDevices.length > 0 ? this.cachedDevices : await this.refreshDevices(true);

    // Refresh in background for next time
    this.refreshDevices(true);

    if (devices.length === 0) {
      vscode.window.showWarningMessage('No devices found. Connect a device or start an emulator.');
      return undefined;
    }

    const currentDevice = this.selectedDevice;
    const availableDevices = devices.filter(d => {
      const state = d.state.toLowerCase();
      return state === 'device' || state === 'available' || state === 'online';
    });
    const offlineSimulators = devices.filter(d => {
      const state = d.state.toLowerCase();
      return (d.type === 'simulator' || d.type === 'emulator') && (state === 'shutdown' || state === 'offline');
    });

    interface DeviceQuickPickItem extends vscode.QuickPickItem {
      deviceId?: string;
      action?: 'select' | 'start';
    }

    const items: DeviceQuickPickItem[] = [];

    // Current Device section
    if (currentDevice) {
      items.push({
        label: `${this.getDeviceIcon(currentDevice)} ${currentDevice.name}`,
        description: currentDevice.platform,
        detail: 'Current Device',
        deviceId: currentDevice.id,
        action: 'select'
      });
    }

    // Available devices
    for (const device of availableDevices) {
      if (currentDevice && device.id === currentDevice.id) continue;

      items.push({
        label: `${this.getDeviceIcon(device)} ${device.name}`,
        description: device.platform,
        deviceId: device.id,
        action: 'select'
      });
    }

    // Offline Emulators
    if (offlineSimulators.length > 0) {
      items.push({ label: '', kind: vscode.QuickPickItemKind.Separator });

      for (const device of offlineSimulators) {
        items.push({
          label: `$(play) ${device.name}`,
          description: device.platform,
          deviceId: device.id,
          action: 'start'
        });
      }
    }

    const selected = await vscode.window.showQuickPick(items, {
      placeHolder: 'Select a device to use'
    }) as DeviceQuickPickItem | undefined;

    if (!selected || !selected.deviceId) {
      return undefined;
    }

    const device = devices.find(d => d.id === selected.deviceId);
    if (!device) return undefined;

    if (selected.action === 'start') {
      await this.startSimulator(device);
      await new Promise(r => setTimeout(r, 3000));
      await this.refreshDevices(true);
    }

    this.setSelectedDevice(device);
    return device;
  }

  private async startSimulator(device: Device): Promise<void> {
    if (device.platform === 'ios') {
      cp.exec(`xcrun simctl boot "${device.id}"`);
    }
  }

  public async ensureDeviceSelected(): Promise<Device | null> {
    if (this.selectedDevice) return this.selectedDevice;

    const devices = await this.refreshDevices(true);
    const activeDevices = devices.filter(d => {
      if (d.platform === 'web') return false;
      const state = d.state.toLowerCase();
      return state === 'device' || state === 'available';
    });

    // Priority: physical first
    const physicalDevices = activeDevices.filter(d => d.type === 'physical');
    if (physicalDevices.length > 0) {
      this.setSelectedDevice(physicalDevices[0]);
      return physicalDevices[0];
    }

    if (activeDevices.length === 1) {
      this.setSelectedDevice(activeDevices[0]);
      return activeDevices[0];
    }

    if (devices.length > 0) {
      return await this.showDevicePicker() || null;
    }

    return null;
  }

  public dispose(): void {
    this.statusBarItem.dispose();
  }
}
