import { strict as assert } from 'node:assert';
import test from 'node:test';
import { parseJigProfileContent } from './jigProfileResolver';

test('parseJigProfileContent parses buttons and multi-channel relays', () => {
  const yamlContent = `
hardware:
  port: "/dev/cu.usbserial-A5069RR4"
  baudrate: 115200
  nodeId: 1
  autoPowerOff: false

  buttons:
    NC1:
      servo: 5
      sensor: 5
    NC2:
      servo: 6
      sensor: 6
    NC3:
      servo: 7
      sensor: 7

  relays:
    mainPower: [3, 4]
    220V: [3, 4]
    KNX: [1]
    24V: [2]
`;

  const profile = parseJigProfileContent(yamlContent, 'test_profile.yaml');

  assert.equal(profile.port, '/dev/cu.usbserial-A5069RR4');
  assert.equal(profile.baudrate, 115200);
  assert.equal(profile.nodeId, 1);
  assert.equal(profile.autoPowerOff, false);

  assert.equal(profile.buttons.size, 3);
  assert.deepEqual(profile.buttons.get('NC1'), { name: 'NC1', servo: 5, sensor: 5, channel: 5 });
  assert.deepEqual(profile.buttons.get('NC2'), { name: 'NC2', servo: 6, sensor: 6, channel: 6 });
  assert.deepEqual(profile.buttons.get('NC3'), { name: 'NC3', servo: 7, sensor: 7, channel: 7 });

  assert.equal(profile.relays.size, 4);
  assert.deepEqual(profile.relays.get('mainPower'), { name: 'mainPower', channels: [3, 4] });
  assert.deepEqual(profile.relays.get('220V'), { name: '220V', channels: [3, 4] });
  assert.deepEqual(profile.relays.get('KNX'), { name: 'KNX', channels: [1] });
  assert.deepEqual(profile.relays.get('24V'), { name: '24V', channels: [2] });
});
