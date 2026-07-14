import * as assert from 'node:assert/strict';
import { test } from 'node:test';
import { parseAdbDevices } from './deviceDiscovery';

test('parses physical emulator offline and unauthorized Android devices', () => {
  const output = `List of devices attached
R5CT123 device product:dm3q model:SM_S918B device:dm3q
emulator-5554 device product:sdk model:sdk_gphone64_x86_64 device:emu64
ABC offline
LOCKED unauthorized usb:1-1
`;

  assert.deepEqual(parseAdbDevices(output), [
    {
      id: 'R5CT123',
      name: 'SM S918B',
      platform: 'android',
      state: 'device',
      type: 'physical'
    },
    {
      id: 'emulator-5554',
      name: 'sdk gphone64 x86 64',
      platform: 'android',
      state: 'device',
      type: 'emulator'
    },
    {
      id: 'ABC',
      name: 'ABC',
      platform: 'android',
      state: 'offline',
      type: 'physical'
    },
    {
      id: 'LOCKED',
      name: 'LOCKED',
      platform: 'android',
      state: 'unauthorized',
      type: 'physical'
    }
  ]);
});

test('ignores adb daemon noise and malformed rows', () => {
  const output = `* daemon started successfully
List of devices attached
not-a-device

`;

  assert.deepEqual(parseAdbDevices(output), []);
});
