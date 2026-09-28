import React, { useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { AlertCircle, Check, Copy, Download, FileCode2, Plus, Puzzle, RefreshCw, ShieldCheck, Trash2, ToggleLeft, ToggleRight } from 'lucide-react';
import { openDialog } from '../utils/tauriUtils';
import { useEditorStore, useFileStore } from '../stores';

interface ExtensionManifest {
  schemaVersion: number;
  lumiApiVersion: number;
  id: string;
  name: string;
  version: string;
  publisher: string;
  description: string;
  contributes: {
    docs: Array<{ title: string; content: string }>;
    snippets: Array<{ prefix: string; description: string; body: string }>;
    templates: Array<{ name: string; fileName: string; content: string }>;
    selectorPacks: Array<{ name: string; description: string; selectors: string[] }>;
    reportViews: Array<{ id: string; label: string; description: string }>;
  };
}

interface InstalledExtension {
  manifest: ExtensionManifest;
  enabled: boolean;
}

const contributionCount = (manifest: ExtensionManifest) => {
  const items = manifest.contributes;
  return items.docs.length + items.snippets.length + items.templates.length + items.selectorPacks.length + items.reportViews.length;
};

export const ExtensionsPanel: React.FC = () => {
  const projectRoot = useFileStore(state => state.projectRoot);
  const [extensions, setExtensions] = useState<InstalledExtension[]>([]);
  const [loading, setLoading] = useState(true);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setExtensions(await invoke<InstalledExtension[]>('list_lumi_extensions'));
    } catch (reason) {
      setError(String(reason));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => { void refresh(); }, [refresh]);

  const installOrUpdate = async () => {
    const selected = await openDialog({
      directory: false,
      multiple: false,
      filters: [{ name: 'Lumi extension manifest', extensions: ['json'] }],
    });
    if (typeof selected !== 'string') return;
    setBusyId('install');
    setError(null);
    try {
      await invoke('install_lumi_extension', { manifestPath: selected });
      await refresh();
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusyId(null);
    }
  };

  const setEnabled = async (extension: InstalledExtension) => {
    setBusyId(extension.manifest.id);
    setError(null);
    try {
      await invoke('set_lumi_extension_enabled', {
        id: extension.manifest.id,
        enabled: !extension.enabled,
      });
      await refresh();
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusyId(null);
    }
  };

  const remove = async (extension: InstalledExtension) => {
    if (!window.confirm(`Remove ${extension.manifest.name}? Its installed manifest and contributions will be removed.`)) return;
    setBusyId(extension.manifest.id);
    setError(null);
    try {
      await invoke('remove_lumi_extension', { id: extension.manifest.id });
      await refresh();
    } catch (reason) {
      setError(String(reason));
    } finally {
      setBusyId(null);
    }
  };

  const copySnippet = async (prefix: string, body: string) => {
    try {
      await navigator.clipboard.writeText(body);
      setNotice(`Copied “${prefix}” to the clipboard.`);
    } catch (reason) {
      setError(`Could not copy snippet: ${String(reason)}`);
    }
  };

  const createTemplate = async (template: ExtensionManifest['contributes']['templates'][number]) => {
    if (!projectRoot) {
      setError('Open a workspace before creating a template file.');
      return;
    }
    setError(null);
    try {
      const path = await invoke<string>('create_workspace_file', {
        parent: projectRoot,
        name: template.fileName,
        content: template.content,
      });
      const fileStore = useFileStore.getState();
      await fileStore.refresh();
      useEditorStore.getState().openFile(path);
      await fileStore.loadContent(path);
      setNotice(`Created ${template.fileName} in the workspace root.`);
    } catch (reason) {
      setError(String(reason));
    }
  };

  return (
    <section className="ide-extension-manager" aria-label="Lumi Extensions">
      <header className="ide-extension-header">
        <div>
          <span className="ide-extension-eyebrow">LUMI IDE</span>
          <h1>Lumi Extensions</h1>
          <p>Install YAML tools and reusable test content from a Lumi extension manifest.</p>
        </div>
        <div className="ide-extension-actions">
          <button type="button" className="ide-extension-refresh" onClick={() => void refresh()} disabled={loading} title="Refresh extensions">
            <RefreshCw size={14} className={loading ? 'is-spinning' : ''} />
          </button>
          <button type="button" className="ide-extension-install" onClick={() => void installOrUpdate()} disabled={busyId === 'install'}>
            <Download size={14} /> {busyId === 'install' ? 'Installing…' : 'Install from file'}
          </button>
        </div>
      </header>

      {error && <div className="ide-extension-error" role="alert"><AlertCircle size={15} /> <span>{error}</span></div>}
      {notice && <div className="ide-extension-notice" role="status"><Check size={14} /> <span>{notice}</span><button type="button" onClick={() => setNotice(null)} aria-label="Dismiss message">Dismiss</button></div>}

      <div className="ide-extension-trust-banner">
        <ShieldCheck size={16} />
        <span>Extensions are declarative JSON. Lumi IDE does not run extension code. Import a newer manifest with the same ID to update.</span>
      </div>

      <div className="ide-extension-list">
        {loading ? (
          <div className="ide-extension-empty"><RefreshCw size={18} className="is-spinning" /> Loading extensions…</div>
        ) : extensions.length === 0 ? (
          <div className="ide-extension-empty">
            <Puzzle size={24} />
            <strong>No Lumi extensions installed</strong>
            <span>Choose a Lumi extension `.json` manifest to install snippets, templates, selector packs, documentation, or report view metadata.</span>
          </div>
        ) : extensions.map(extension => {
          const { manifest } = extension;
          const isBusy = busyId === manifest.id;
          const count = contributionCount(manifest);
          return (
            <article className={`ide-extension-card ${extension.enabled ? '' : 'is-disabled'}`} key={manifest.id}>
              <div className="ide-extension-card-top">
                <div className="ide-extension-icon"><Puzzle size={17} /></div>
                <div className="ide-extension-card-title">
                  <strong>{manifest.name}</strong>
                  <span>{manifest.publisher ? `${manifest.publisher} · ` : ''}{manifest.id} · v{manifest.version}</span>
                </div>
                <span className={`ide-extension-state ${extension.enabled ? 'is-enabled' : ''}`}>
                  {extension.enabled ? <><Check size={12} /> Enabled</> : 'Disabled'}
                </span>
              </div>
              <p className="ide-extension-description">{manifest.description || 'No description provided.'}</p>
              <div className="ide-extension-meta">
                <span>API {manifest.lumiApiVersion}</span>
                <span>{count} contribution{count === 1 ? '' : 's'}</span>
                <span>{manifest.contributes.snippets.length} snippets</span>
                <span>{manifest.contributes.templates.length} templates</span>
              </div>

              {count > 0 && (
                <details className="ide-extension-contributions">
                  <summary><FileCode2 size={13} /> View contributions</summary>
                  <div className="ide-extension-contribution-list">
                    {manifest.contributes.docs.map((doc, index) => <details key={`doc-${index}`}><summary>Docs · {doc.title}</summary><pre>{doc.content}</pre></details>)}
                    {manifest.contributes.snippets.map((snippet, index) => <details key={`snippet-${index}`}><summary>Snippet · {snippet.prefix}</summary><p>{snippet.description}</p><pre>{snippet.body}</pre><button type="button" className="ide-extension-contribution-action" disabled={!extension.enabled} onClick={() => void copySnippet(snippet.prefix, snippet.body)}><Copy size={12} /> Copy snippet</button></details>)}
                    {manifest.contributes.templates.map((template, index) => <details key={`template-${index}`}><summary>Template · {template.name} ({template.fileName})</summary><pre>{template.content}</pre><button type="button" className="ide-extension-contribution-action" disabled={!extension.enabled || !projectRoot} onClick={() => void createTemplate(template)}><Plus size={12} /> Create in workspace</button></details>)}
                    {manifest.contributes.selectorPacks.map((pack, index) => <details key={`selectors-${index}`}><summary>Selector pack · {pack.name}</summary><p>{pack.description}</p><ul>{pack.selectors.map((selector, selectorIndex) => <li key={selectorIndex}><code>{selector}</code></li>)}</ul></details>)}
                    {manifest.contributes.reportViews.map(view => <div className="ide-extension-report-view" key={view.id}><strong>Report view · {view.label}</strong><span>{view.description}</span></div>)}
                  </div>
                </details>
              )}

              <footer className="ide-extension-card-actions">
                <button type="button" onClick={() => void setEnabled(extension)} disabled={isBusy}>
                  {extension.enabled ? <ToggleRight size={15} /> : <ToggleLeft size={15} />}
                  {extension.enabled ? 'Disable' : 'Enable'}
                </button>
                <button type="button" className="is-danger" onClick={() => void remove(extension)} disabled={isBusy}>
                  <Trash2 size={14} /> Remove
                </button>
              </footer>
            </article>
          );
        })}
      </div>

      <div className="ide-extension-format-note">
        <span>Manifest format: <code>schemaVersion: 1</code> · <code>lumiApiVersion: 1</code></span>
        <span>Installing a higher semantic version for an existing ID updates that extension.</span>
      </div>
    </section>
  );
};
