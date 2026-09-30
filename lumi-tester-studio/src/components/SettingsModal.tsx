import React, { useCallback, useEffect, useState } from 'react';
import { useEnvStore } from '../stores';
import { X, Plus, Trash2, Save, RefreshCw, Download, ExternalLink, CheckCircle2, ArrowUpCircle } from 'lucide-react';
import { openUrl } from '@tauri-apps/plugin-opener';
import packageJson from '../../package.json';
import { compareVersions, fetchLatestIdeRelease, IdeRelease } from '../services/updateService';

interface Props {
  onClose: () => void;
}

export const SettingsModal: React.FC<Props> = ({ onClose }) => {
  const { envVars, setEnvVars } = useEnvStore();
  const [localVars, setLocalVars] = useState(envVars);
  const [activeTab, setActiveTab] = useState<'environment' | 'about'>('environment');
  const [release, setRelease] = useState<IdeRelease | null>(null);
  const [checkState, setCheckState] = useState<'idle' | 'checking' | 'ready' | 'error'>('idle');
  const [checkError, setCheckError] = useState('');

  const checkForUpdates = useCallback(async () => {
    setCheckState('checking');
    setCheckError('');
    try {
      const result = await fetchLatestIdeRelease();
      setRelease(result.latest);
      setCheckState('ready');
    } catch (error) {
      setCheckError(error instanceof Error ? error.message : String(error));
      setCheckState('error');
    }
  }, []);

  useEffect(() => {
    if (activeTab === 'about' && checkState === 'idle') void checkForUpdates();
  }, [activeTab, checkForUpdates, checkState]);

  const handleSave = () => {
    setEnvVars(localVars);
    onClose();
  };

  const addVar = () => {
    setLocalVars([...localVars, { key: '', value: '', enabled: true }]);
  };

  const removeVar = (idx: number) => {
    setLocalVars(localVars.filter((_, i) => i !== idx));
  };

  const updateVar = (idx: number, field: keyof typeof localVars[0], value: any) => {
    const newVars = [...localVars];
    newVars[idx] = { ...newVars[idx], [field]: value };
    setLocalVars(newVars);
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/80 backdrop-blur-sm p-4">
      <div className="bg-slate-900 border border-borderGlass rounded-xl shadow-2xl w-full max-w-2xl flex flex-col max-h-[80vh]">
        <div className="flex items-center justify-between p-6 border-b border-borderGlass">
          <div>
            <h2 className="text-lg font-bold text-white">Settings</h2>
            <p className="text-xs text-slate-500 mt-1">Configure Lumi IDE and check for updates</p>
          </div>
          <button onClick={onClose} className="text-slate-400 hover:text-white transition-colors">
            <X size={20} />
          </button>
        </div>

        <div className="flex gap-2 px-6 pt-4 border-b border-borderGlass">
          <button
            onClick={() => setActiveTab('environment')}
            className={`px-3 py-2 text-sm border-b-2 transition-colors ${activeTab === 'environment' ? 'text-cyan-300 border-cyan-400' : 'text-slate-400 border-transparent hover:text-white'}`}
          >
            Environment
          </button>
          <button
            onClick={() => setActiveTab('about')}
            className={`px-3 py-2 text-sm border-b-2 transition-colors ${activeTab === 'about' ? 'text-cyan-300 border-cyan-400' : 'text-slate-400 border-transparent hover:text-white'}`}
          >
            About & Updates
          </button>
        </div>

        <div className="p-6 overflow-y-auto flex-1">
          {activeTab === 'environment' ? <>
          <p className="text-sm text-slate-500 mb-4">
            Variables defined here can be used in your YAML tests using
            <span className="font-mono text-cyan-400 mx-1">{'{{key}}'}</span> syntax.
            Responses from requests can also update these variables automatically.
          </p>

          <div className="space-y-3">
            {localVars.map((v, idx) => (
              <div key={idx} className="flex items-center gap-3 animate-in fade-in slide-in-from-left-4 duration-300">
                <input
                  type="checkbox"
                  checked={v.enabled}
                  onChange={(e) => updateVar(idx, 'enabled', e.target.checked)}
                  className="w-4 h-4 rounded border-slate-600 bg-slate-800 text-cyan-500 focus:ring-offset-slate-900"
                />
                <input
                  type="text"
                  placeholder="Key"
                  value={v.key}
                  onChange={(e) => updateVar(idx, 'key', e.target.value)}
                  className="ide-form-control text-sm flex-1"
                />
                <div className="text-slate-600">=</div>
                <input
                  type="text"
                  placeholder="Value"
                  value={v.value}
                  onChange={(e) => updateVar(idx, 'value', e.target.value)}
                  className="ide-form-control ide-form-control--accent-text text-sm flex-1 font-mono"
                />
                <button
                  onClick={() => removeVar(idx)}
                  className="p-2 text-rose-500 hover:bg-rose-900/20 rounded transition-colors"
                >
                  <Trash2 size={16} />
                </button>
              </div>
            ))}
          </div>

          <button
            onClick={addVar}
            className="mt-4 flex items-center gap-2 text-sm text-cyan-400 hover:text-cyan-300 font-medium transition-colors"
          >
            <Plus size={16} /> Add Variable
          </button>
          </> : <div className="space-y-5">
            <section className="rounded-xl border border-slate-700 bg-slate-950/60 p-5">
              <div className="flex flex-wrap items-center justify-between gap-4">
                <div className="flex items-center gap-3">
                  <div className="flex h-11 w-11 items-center justify-center rounded-xl bg-cyan-500/10 text-cyan-300">
                    <span className="text-lg font-bold">L</span>
                  </div>
                  <div>
                    <h3 className="font-semibold text-white">Lumi IDE</h3>
                    <p className="text-sm text-slate-400">Version {packageJson.version}</p>
                  </div>
                </div>
                <button
                  type="button"
                  onClick={() => void checkForUpdates()}
                  disabled={checkState === 'checking'}
                  className="inline-flex items-center gap-2 rounded-lg border border-cyan-500/40 bg-cyan-500/10 px-3 py-2 text-sm font-medium text-cyan-200 transition-colors hover:bg-cyan-500/20 disabled:cursor-wait disabled:opacity-60"
                >
                  <RefreshCw size={15} className={checkState === 'checking' ? 'animate-spin' : ''} />
                  {checkState === 'checking' ? 'Checking…' : 'Check for Updates'}
                </button>
              </div>

              {checkState === 'error' && (
                <p role="status" className="mt-4 rounded-lg border border-rose-500/30 bg-rose-500/10 px-3 py-2 text-sm text-rose-200">
                  {checkError}
                </p>
              )}

              {checkState === 'ready' && release && (
                <div role="status" className="mt-4 flex flex-wrap items-center justify-between gap-3 rounded-lg border border-slate-700 bg-slate-900 px-3 py-3">
                  {compareVersions(release.version, packageJson.version) > 0 ? (
                    <div className="flex items-center gap-2 text-sm text-cyan-200">
                      <ArrowUpCircle size={17} />
                      <span>Version {release.version} is available</span>
                    </div>
                  ) : (
                    <div className="flex items-center gap-2 text-sm text-emerald-300">
                      <CheckCircle2 size={17} />
                      <span>You’re up to date</span>
                    </div>
                  )}
                  {compareVersions(release.version, packageJson.version) > 0 && (
                    <button
                      type="button"
                      onClick={() => void openUrl(release.url).catch(error => window.alert(`Could not open release page: ${String(error)}`))}
                      className="inline-flex items-center gap-2 rounded-md bg-cyan-600 px-3 py-1.5 text-sm font-medium text-white hover:bg-cyan-500"
                    >
                      <Download size={15} /> Download
                      <ExternalLink size={13} />
                    </button>
                  )}
                </div>
              )}
            </section>

            <section className="rounded-xl border border-slate-700 bg-slate-950/40 p-5">
              <div className="mb-3 flex items-center justify-between gap-3">
                <div>
                  <h3 className="font-semibold text-white">Release Notes</h3>
                  <p className="text-xs text-slate-500 mt-1">{release ? release.name : 'Check for updates to load the latest changelog.'}</p>
                </div>
                {release && <button
                  type="button"
                  onClick={() => void openUrl(release.url).catch(error => window.alert(`Could not open release page: ${String(error)}`))}
                  className="text-slate-400 hover:text-cyan-300"
                  title="Open release notes"
                >
                  <ExternalLink size={16} />
                </button>}
              </div>
              {release ? (
                <pre className="max-h-64 overflow-auto whitespace-pre-wrap break-words font-sans text-sm leading-6 text-slate-300">{release.notes}</pre>
              ) : (
                <p className="text-sm text-slate-500">Release notes will appear here after checking for updates.</p>
              )}
            </section>

            <p className="text-xs leading-5 text-slate-500">
              Update checks use Lumi IDE’s GitHub releases. Downloads open the release page; signed in-app installation is not enabled in this build.
            </p>
          </div>}
        </div>

        {activeTab === 'environment' && <div className="p-6 border-t border-borderGlass bg-slate-900/50 flex justify-end gap-3 rounded-b-xl">
          <button onClick={onClose} className="px-4 py-2 text-sm font-medium text-slate-400 hover:text-white transition-colors">Cancel</button>
          <button onClick={handleSave} className="px-6 py-2 bg-cyan-600 hover:bg-cyan-500 text-white text-sm font-medium rounded-lg shadow-lg hover:shadow-cyan-500/20 transition-all flex items-center gap-2">
            <Save size={16} /> Save Changes
          </button>
        </div>}
      </div>
    </div>
  );
};
