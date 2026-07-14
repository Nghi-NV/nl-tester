export interface AndroidDevice {
  id: string;
  name: string;
  platform: 'android';
  state: string;
  type: 'physical' | 'emulator';
}

export function parseAdbDevices(output: string): AndroidDevice[] {
  const devices: AndroidDevice[] = [];
  for (const line of output.split(/\r?\n/)) {
    if (!line.trim()) continue;
    const match = line.match(/^(\S+)\s+(device|offline|unauthorized)(?:\s+.*?model:(\S+))?/);
    if (!match) continue;

    const id = match[1];
    devices.push({
      id,
      name: (match[3] ?? id).replace(/_/g, ' '),
      platform: 'android',
      state: match[2],
      type: id.startsWith('emulator-') ? 'emulator' : 'physical'
    });
  }
  return devices;
}
