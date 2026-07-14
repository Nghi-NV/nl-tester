import * as assert from 'node:assert/strict';
import { test } from 'node:test';
import { buildRunInvocation } from './commandInvocation';

test('runs a configured Windows executable directly', () => {
  const invocation = buildRunInvocation({
    lumiPath: 'C:\\Users\\buith\\.lumi-tester\\bin\\lumi-tester.exe',
    lumiPathIsFile: true,
    testFilePath: 'C:\\tests\\login.yaml',
    commandIndex: 3,
    device: { platform: 'android', id: 'emulator-5554' }
  });

  assert.deepEqual(invocation, {
    executable: 'C:\\Users\\buith\\.lumi-tester\\bin\\lumi-tester.exe',
    args: [
      'run',
      'C:\\tests\\login.yaml',
      '--command-index',
      '3',
      '--platform',
      'android',
      '--device',
      'emulator-5554'
    ]
  });
});

test('runs cargo from a source directory', () => {
  const invocation = buildRunInvocation({
    lumiPath: '/workspace/lumi-tester',
    lumiPathIsFile: false,
    testFilePath: '/workspace/tests/login.yaml'
  });

  assert.deepEqual(invocation, {
    executable: 'cargo',
    args: ['run', '--', 'run', '/workspace/tests/login.yaml'],
    cwd: '/workspace/lumi-tester'
  });
});
