import * as cp from 'child_process';
import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';

export interface AppInfo {
  name: string;
  bundleId?: string;
  path: string;
  iconPath?: string;
  isRunning?: boolean;
}

/**
 * Find the primary .icns icon file inside a macOS .app bundle
 */
export function findAppIconFile(appPath: string): string | undefined {
  try {
    const resDir = path.join(appPath, 'Contents', 'Resources');
    if (fs.existsSync(resDir)) {
      const files = fs.readdirSync(resDir);
      const icns = files.find(f => f.toLowerCase().endsWith('.icns'));
      if (icns) {
        return path.join(resDir, icns);
      }
    }
  } catch {
    // ignore
  }
  return undefined;
}

/**
 * Discover installed and running macOS applications
 */
export async function getMacosApplications(): Promise<AppInfo[]> {
  if (process.platform !== 'darwin') return [];

  const script = `
import AppKit
import Foundation

var seen = Set<String>()
var apps: [String] = []

// 1. Running GUI Apps first
for app in NSWorkspace.shared.runningApplications {
    guard app.activationPolicy == .regular else { continue }
    let name = app.localizedName ?? ""
    let bId = app.bundleIdentifier ?? ""
    let path = app.bundleURL?.path ?? ""
    if !path.isEmpty && !seen.contains(path) {
        seen.insert(path)
        if !bId.isEmpty {
            apps.append("\(name) [Running] | \(bId) | \(path)")
        } else {
            apps.append("\(name) [Running] | \(path) | \(path)")
        }
    }
}

// 2. Installed Apps in standard directories
let dirs = [
    "/Applications",
    "/System/Applications",
    "/System/Applications/Utilities",
    "\\(NSHomeDirectory())/Applications"
]

let fm = FileManager.default
for dir in dirs {
    guard let contents = try? fm.contentsOfDirectory(atPath: dir) else { continue }
    for item in contents where item.hasSuffix(".app") {
        let fullPath = (dir as NSString).appendingPathComponent(item)
        if !seen.contains(fullPath) {
            seen.insert(fullPath)
            let bundle = Bundle(path: fullPath)
            let bId = bundle?.bundleIdentifier ?? ""
            let name = (item as NSString).deletingPathExtension
            if !bId.isEmpty {
                apps.append("\(name) | \(bId) | \(fullPath)")
            } else {
                apps.append("\(name) | \(fullPath) | \(fullPath)")
            }
        }
    }
}

for app in apps {
    print(app)
}
`;

  return new Promise((resolve) => {
    cp.execFile('swift', ['-e', script], { timeout: 10000 }, (error, stdout) => {
      if (error) {
        resolve(scanMacosDirectoriesFallback());
        return;
      }

      const results: AppInfo[] = [];
      const lines = stdout.split('\n');
      for (const line of lines) {
        if (!line.includes('|')) continue;
        const parts = line.split('|').map(s => s.trim());
        const namePart = parts[0] || '';
        const isRunning = namePart.includes('[Running]');
        const name = namePart.replace('[Running]', '').trim();
        const bundleId = parts[1] || '';
        const appPath = parts[2] || '';
        if (name && appPath) {
          results.push({
            name,
            bundleId: bundleId || undefined,
            path: appPath,
            iconPath: findAppIconFile(appPath),
            isRunning
          });
        }
      }
      resolve(results);
    });
  });
}

function scanMacosDirectoriesFallback(): AppInfo[] {
  const dirs = [
    '/Applications',
    '/System/Applications',
    '/System/Applications/Utilities',
    path.join(os.homedir(), 'Applications')
  ];

  const results: AppInfo[] = [];
  const seen = new Set<string>();

  for (const dir of dirs) {
    if (!fs.existsSync(dir)) continue;
    try {
      const items = fs.readdirSync(dir);
      for (const item of items) {
        if (item.endsWith('.app')) {
          const fullPath = path.join(dir, item);
          if (!seen.has(fullPath)) {
            seen.add(fullPath);
            results.push({
              name: item.replace(/\.app$/, ''),
              path: fullPath,
              iconPath: findAppIconFile(fullPath),
              isRunning: false
            });
          }
        }
      }
    } catch {
      // Ignore permission or read errors
    }
  }

  return results;
}
