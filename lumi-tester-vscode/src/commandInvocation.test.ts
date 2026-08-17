import * as assert from 'node:assert/strict';
import { test } from 'node:test';
import { buildInspectInvocation, buildRunInvocation } from './commandInvocation';

test('runs a configured Windows executable directly', () => {
  const invocation = buildRunInvocation({
    runtime: {
      kind: 'binary',
      executable: 'C:\\Users\\QueDT\\.lumi-tester\\bin\\lumi-tester.exe',
      argsPrefix: []
    },
    testFilePath: 'C:\\tests\\login.yaml',
    commandIndex: 3,
    device: { platform: 'android', id: 'emulator-5554' }
  });

  assert.deepEqual(invocation, {
    executable: 'C:\\Users\\QueDT\\.lumi-tester\\bin\\lumi-tester.exe',
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

test('builds run invocation with fromCommandIndex', () => {
  const invocation = buildRunInvocation({
    runtime: {
      kind: 'binary',
      executable: '/usr/local/bin/lumi-tester',
      argsPrefix: []
    },
    testFilePath: '/workspace/tests/login.yaml',
    fromCommandIndex: 2,
    device: { platform: 'android', id: 'ADE00005891' }
  });

  assert.deepEqual(invocation, {
    executable: '/usr/local/bin/lumi-tester',
    args: [
      'run',
      '/workspace/tests/login.yaml',
      '--from-command-index',
      '2',
      '--platform',
      'android',
      '--device',
      'ADE00005891'
    ]
  });
});

test('keeps cargo only for a source runtime', () => {
  const invocation = buildRunInvocation({
    runtime: {
      kind: 'source',
      executable: 'cargo',
      argsPrefix: ['run', '--'],
      cwd: '/workspace/lumi-tester'
    },
    testFilePath: '/workspace/tests/login.yaml'
  });

  assert.deepEqual(invocation, {
    executable: 'cargo',
    args: ['run', '--', 'run', '/workspace/tests/login.yaml'],
    cwd: '/workspace/lumi-tester'
  });
});

test('builds inspector arguments for an installed binary', () => {
  const invocation = buildInspectInvocation({
    runtime: {
      kind: 'binary',
      executable: 'C:\\Users\\QueDT\\.lumi-tester\\bin\\lumi-tester.exe',
      argsPrefix: []
    },
    platform: 'android',
    port: 9333,
    deviceId: 'R5CT123'
  });

  assert.deepEqual(invocation, {
    executable: 'C:\\Users\\QueDT\\.lumi-tester\\bin\\lumi-tester.exe',
    args: ['inspect', '--platform', 'android', '--port', '9333', '--device', 'R5CT123']
  });
});

test('builds inspector arguments for a source runtime', () => {
  const invocation = buildInspectInvocation({
    runtime: {
      kind: 'source',
      executable: 'cargo',
      argsPrefix: ['run', '--'],
      cwd: '/workspace/lumi-tester'
    },
    platform: 'ios',
    port: 9334
  });

  assert.deepEqual(invocation, {
    executable: 'cargo',
    args: ['run', '--', 'inspect', '--platform', 'ios', '--port', '9334'],
    cwd: '/workspace/lumi-tester'
  });
});
