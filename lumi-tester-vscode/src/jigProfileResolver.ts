import * as fs from 'fs';
import * as path from 'path';

export interface JigButtonConfig {
  name: string;
  servo?: number;
  sensor?: number;
  channel?: number;
}

export interface JigRelayConfig {
  name: string;
  channels: number[];
}

export interface JigProfileData {
  port?: string;
  baudrate?: number;
  nodeId?: number;
  autoPowerOff?: boolean;
  buttons: Map<string, JigButtonConfig>;
  relays: Map<string, JigRelayConfig>;
  sourceFile?: string;
}

interface CacheEntry {
  mtimeMs: number;
  data: JigProfileData;
}

const fileCache = new Map<string, CacheEntry>();

/**
 * Parse a simple YAML file into JigProfileData without heavy third-party dependencies.
 */
export function parseJigProfileContent(content: string, sourceFile?: string): JigProfileData {
  const buttons = new Map<string, JigButtonConfig>();
  const relays = new Map<string, JigRelayConfig>();

  let port: string | undefined;
  let baudrate: number | undefined;
  let nodeId: number | undefined;
  let autoPowerOff: boolean | undefined;

  const lines = content.split(/\r?\n/);
  let currentSection: 'root' | 'buttons' | 'relays' | 'servos' | 'other' = 'root';
  let currentButtonName: string | undefined;

  for (let i = 0; i < lines.length; i++) {
    const rawLine = lines[i];
    // Strip comments
    const commentIdx = rawLine.indexOf('#');
    const line = commentIdx >= 0 ? rawLine.substring(0, commentIdx) : rawLine;
    const trimmed = line.trim();
    if (!trimmed) {
      continue;
    }

    const indent = rawLine.match(/^(\s*)/)?.[1].length ?? 0;

    // Detect section headers
    if (indent === 0 || indent === 2) {
      if (trimmed.startsWith('buttons:')) {
        currentSection = 'buttons';
        currentButtonName = undefined;
        continue;
      }
      if (trimmed.startsWith('relays:')) {
        currentSection = 'relays';
        currentButtonName = undefined;
        continue;
      }
      if (trimmed.startsWith('servos:')) {
        currentSection = 'servos';
        currentButtonName = undefined;
        continue;
      }
      if (trimmed.startsWith('hardware:') || trimmed.startsWith('jig:')) {
        currentSection = 'root';
        continue;
      }
      if (indent === 0 && !trimmed.startsWith(' ') && !trimmed.startsWith('-')) {
        // Other root level key
        const colonIdx = trimmed.indexOf(':');
        if (colonIdx > 0) {
          const key = trimmed.substring(0, colonIdx).trim();
          if (key !== 'buttons' && key !== 'relays' && key !== 'servos') {
            currentSection = 'other';
          }
        }
      }
    }

    // Parse root/hardware properties
    const portMatch = trimmed.match(/^port:\s*["']?([^"'\s]+)["']?/);
    if (portMatch) {
      port = portMatch[1];
    }
    const baudMatch = trimmed.match(/^baudrate:\s*(\d+)/);
    if (baudMatch) {
      baudrate = parseInt(baudMatch[1], 10);
    }
    const nodeMatch = trimmed.match(/^nodeId:\s*(\d+)/);
    if (nodeMatch) {
      nodeId = parseInt(nodeMatch[1], 10);
    }
    const autoOffMatch = trimmed.match(/^autoPowerOff:\s*(true|false)/i);
    if (autoOffMatch) {
      autoPowerOff = autoOffMatch[1].toLowerCase() === 'true';
    }

    // Parse buttons
    if (currentSection === 'buttons') {
      const reservedButtonKeys = new Set([
        'servo', 'sensor', 'channel', 'pressAngle', 'releaseAngle',
        'pressDurationMs', 'releaseDurationMs', 'holdDurationMs',
        'gain', 'channels', 'minPulseMs', 'maxPulseMs'
      ]);

      // Check sub-properties of button e.g. "servo: 5", "sensor: 5"
      const subPropMatch = line.match(/^\s{4,8}([A-Za-z0-9_\-]+):\s*(\d+)/);
      if (subPropMatch && currentButtonName && buttons.has(currentButtonName)) {
        const prop = subPropMatch[1];
        const val = parseInt(subPropMatch[2], 10);
        const btn = buttons.get(currentButtonName)!;
        if (prop === 'servo') {
          btn.servo = val;
          if (btn.channel === undefined) {
            btn.channel = val;
          }
        } else if (prop === 'sensor') {
          btn.sensor = val;
          if (btn.channel === undefined) {
            btn.channel = val;
          }
        } else if (prop === 'channel') {
          btn.channel = val;
        }
        continue;
      }

      // Check for button name e.g. "NC1:" or "NC1: 5"
      const btnHeaderMatch = line.match(/^(\s{2,6})([A-Za-z0-9_\-]+):\s*(.*)$/);
      if (btnHeaderMatch) {
        const name = btnHeaderMatch[2];
        if (reservedButtonKeys.has(name)) {
          continue;
        }
        const rest = btnHeaderMatch[3].trim();

        if (rest && !isNaN(Number(rest))) {
          // Simple "NC1: 5" format (channel 5 for both servo & sensor)
          const ch = Number(rest);
          buttons.set(name, { name, servo: ch, sensor: ch, channel: ch });
          currentButtonName = undefined;
        } else {
          currentButtonName = name;
          if (!buttons.has(name)) {
            buttons.set(name, { name });
          }
        }
        continue;
      }
    }

    // Parse relays e.g. "mainPower: [3, 4]" or "KNX: [1]" or "220V: 3"
    if (currentSection === 'relays') {
      const relayMatch = line.match(/^\s{2,6}([A-Za-z0-9_\-]+):\s*(.+)$/);
      if (relayMatch) {
        const name = relayMatch[1];
        const valStr = relayMatch[2].trim();

        let channels: number[] = [];
        if (valStr.startsWith('[') && valStr.endsWith(']')) {
          const inner = valStr.substring(1, valStr.length - 1);
          channels = inner
            .split(',')
            .map(s => parseInt(s.trim(), 10))
            .filter(n => !isNaN(n));
        } else if (!isNaN(Number(valStr))) {
          channels = [Number(valStr)];
        }

        if (channels.length > 0) {
          relays.set(name, { name, channels });
        }
      }
    }
  }

  return {
    port,
    baudrate,
    nodeId,
    autoPowerOff,
    buttons,
    relays,
    sourceFile,
  };
}

export function resolveJigProfileFromTextAndPath(
  text: string,
  docFsPath: string,
  workspaceRoot?: string
): JigProfileData | undefined {
  const sepIdx = text.indexOf('---');
  const header = sepIdx !== -1 ? text.substring(0, sepIdx) : text;

  // 1. Check if header has inline buttons/relays
  if (header.includes('buttons:') || header.includes('relays:')) {
    const inlineData = parseJigProfileContent(header, docFsPath);
    if (inlineData.buttons.size > 0 || inlineData.relays.size > 0) {
      return inlineData;
    }
  }

  // 2. Check for jig profile file reference e.g. jig: "jig_profile.yaml" or jig: jig_profile.yaml or file: "..."
  const jigMatch = header.match(/^\s*jig:\s*["']?([^"'\r\n]+)["']?/m);
  let relPath: string | undefined;

  if (jigMatch) {
    const matchedVal = jigMatch[1].trim();
    if (matchedVal.endsWith('.yaml') || matchedVal.endsWith('.yml')) {
      relPath = matchedVal;
    }
  }

  if (!relPath) {
    const fileMatch = header.match(/^\s*file:\s*["']?([^"'\r\n]+)["']?/m);
    if (fileMatch && (fileMatch[1].endsWith('.yaml') || fileMatch[1].endsWith('.yml'))) {
      relPath = fileMatch[1].trim();
    }
  }

  if (!relPath) {
    return undefined;
  }

  // Resolve absolute path
  const docDir = path.dirname(docFsPath);
  const possiblePaths = [
    path.resolve(docDir, relPath),
    workspaceRoot ? path.resolve(workspaceRoot, relPath) : undefined,
  ].filter((p): p is string => Boolean(p));

  for (const candidate of possiblePaths) {
    if (fs.existsSync(candidate)) {
      try {
        const stat = fs.statSync(candidate);
        const cached = fileCache.get(candidate);
        if (cached && cached.mtimeMs === stat.mtimeMs) {
          return cached.data;
        }

        const content = fs.readFileSync(candidate, 'utf-8');
        const data = parseJigProfileContent(content, candidate);
        fileCache.set(candidate, { mtimeMs: stat.mtimeMs, data });
        return data;
      } catch (err) {
        console.error('Failed to read jig profile file:', candidate, err);
      }
    }
  }

  return undefined;
}

/**
 * Resolve JigProfileData from the current active YAML document (looking at header jig: reference or inline declaration).
 */
export function resolveJigProfileFromDocument(document: { getText(): string; uri: { fsPath: string } }): JigProfileData | undefined {
  let workspaceRoot: string | undefined;
  try {
    const vscode = require('vscode');
    workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
  } catch {
    // running in plain node test
  }
  return resolveJigProfileFromTextAndPath(document.getText(), document.uri.fsPath, workspaceRoot);
}
