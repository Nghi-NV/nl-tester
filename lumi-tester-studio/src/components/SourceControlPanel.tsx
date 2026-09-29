import React, { useCallback, useEffect, useState } from 'react';
import { Check, ChevronDown, CircleAlert, FileCode2, GitBranch, Loader2, Minus, Plus, RefreshCw, Send } from 'lucide-react';
import { useFileStore } from '../stores';
import { getSourceControlDiff, getSourceControlStatus, SourceControlFile, SourceControlSnapshot, stageSourceControlFile, commitSourceControlChanges, switchSourceControlBranch, initializeSourceControl } from '../utils/tauriUtils';

export const SourceControlPanel: React.FC = () => {
  const workspacePath = useFileStore(state => state.projectRoot);
  const [snapshot, setSnapshot] = useState<SourceControlSnapshot | null>(null);
  const [selected, setSelected] = useState<SourceControlFile | null>(null);
  const [showStaged, setShowStaged] = useState(false);
  const [diff, setDiff] = useState('');
  const [diffTruncated, setDiffTruncated] = useState(false);
  const [message, setMessage] = useState('');
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!workspacePath) {
      setSnapshot(null);
      setSelected(null);
      return;
    }
    setLoading(true);
    setError(null);
    try {
      const next = await getSourceControlStatus(workspacePath);
      setSnapshot(next);
      setSelected(current => current ? next.files.find(file => file.path === current.path) ?? null : next.files[0] ?? null);
    } catch (refreshError) {
      setError(String(refreshError));
      setSnapshot(null);
      setSelected(null);
    } finally {
      setLoading(false);
    }
  }, [workspacePath]);

  useEffect(() => { void refresh(); }, [refresh]);

  useEffect(() => {
    let cancelled = false;
    if (!workspacePath || !selected) {
      setDiff('');
      return;
    }
    const staged = selected.staged && (showStaged || !selected.modified);
    void getSourceControlDiff(workspacePath, selected.path, staged).then(result => {
      if (cancelled) return;
      setDiff(result.text);
      setDiffTruncated(result.truncated);
    }).catch(diffError => {
      if (!cancelled) setDiff(String(diffError));
    });
    return () => { cancelled = true; };
  }, [workspacePath, selected, showStaged]);

  const updateIndex = async (file: SourceControlFile, stage: boolean) => {
    if (!workspacePath) return;
    setBusy(true);
    setError(null);
    try {
      setShowStaged(stage);
      await stageSourceControlFile(workspacePath, file.path, stage);
      await refresh();
    } catch (stageError) {
      setError(String(stageError));
    } finally {
      setBusy(false);
    }
  };

  const commit = async () => {
    if (!workspacePath || !message.trim()) return;
    setBusy(true);
    setError(null);
    setNotice(null);
    try {
      const result = await commitSourceControlChanges(workspacePath, message);
      setMessage('');
      setNotice(result || 'Commit created.');
      await refresh();
    } catch (commitError) {
      setError(String(commitError));
    } finally {
      setBusy(false);
    }
  };

  const switchBranch = async (branch: string) => {
    if (!workspacePath || !branch || branch === snapshot?.branch) return;
    if (snapshot?.files.length && !window.confirm(`Switch to ${branch} with ${snapshot.files.length} changed file(s)? Git will stop if the changes conflict.`)) return;
    setBusy(true);
    setError(null);
    try {
      await switchSourceControlBranch(workspacePath, branch);
      setShowStaged(false);
      await refresh();
    } catch (branchError) {
      setError(String(branchError));
    } finally {
      setBusy(false);
    }
  };

  const initRepository = async () => {
    if (!workspacePath) return;
    setBusy(true);
    setError(null);
    try {
      await initializeSourceControl(workspacePath);
      await refresh();
    } catch (initError) {
      setError(String(initError));
    } finally {
      setBusy(false);
    }
  };

  const stagedFiles = snapshot?.files.filter(file => file.staged) ?? [];
  const unstagedFiles = snapshot?.files.filter(file => !file.staged || file.modified || file.untracked) ?? [];

  const fileRow = (file: SourceControlFile, staged: boolean) => (
    <div key={`${staged ? 'staged' : 'working'}:${file.path}`} className={`ide-git-file${selected?.path === file.path ? ' is-active' : ''}`}>
      <button type="button" className="ide-git-file-main" title={file.path} onClick={() => { setSelected(file); setShowStaged(staged); }}>
        <FileCode2 size={13} />
        <span>{file.path}</span>
        <small className={file.conflict ? 'is-conflict' : file.untracked ? 'is-untracked' : ''}>
          {file.conflict ? '!' : file.untracked ? 'U' : file.deleted ? 'D' : file.staged ? 'M' : 'M'}
        </small>
      </button>
      <button type="button" className="ide-git-file-action" title={staged ? 'Unstage file' : 'Stage file'} disabled={busy} onClick={() => void updateIndex(file, !staged)}>
        {staged ? <Minus size={13} /> : <Plus size={13} />}
      </button>
    </div>
  );

  return (
    <section className="ide-side-view ide-source-control" aria-label="Source Control">
      <header className="ide-side-view-header">
        <span>SOURCE CONTROL</span>
        <button type="button" title="Refresh Git status" aria-label="Refresh Git status" disabled={loading} onClick={() => void refresh()}>
          {loading ? <Loader2 size={13} className="animate-spin" /> : <RefreshCw size={13} />}
        </button>
      </header>
      {!workspacePath && <p className="ide-view-empty">Open a workspace to view Git changes.</p>}
      {error && <div className="ide-view-error ide-git-error"><CircleAlert size={13} /><span>{error}</span>{error.toLowerCase().includes('not a git repository') && workspacePath && <button type="button" disabled={busy} onClick={() => void initRepository()}>Initialize Repository</button>}</div>}
      {notice && <p className="ide-git-notice"><Check size={13} />{notice}</p>}
      {snapshot && (
        <>
          <div className="ide-git-branch"><GitBranch size={13} />
            <select value={snapshot.branch} disabled={busy || loading || snapshot.branches.length === 0} onChange={event => void switchBranch(event.target.value)} aria-label="Current Git branch">
              {!snapshot.branch && <option value="">Detached HEAD</option>}
              {snapshot.branches.map(branch => <option value={branch} key={branch}>{branch}</option>)}
            </select>
            <button type="button" title="Refresh branch" onClick={() => void refresh()}><RefreshCw size={12} /></button>
          </div>
          <div className="ide-git-commit-box">
            <textarea value={message} onChange={event => setMessage(event.target.value)} placeholder="Commit message" aria-label="Commit message" rows={2} />
            <button type="button" disabled={busy || stagedFiles.length === 0 || !message.trim()} onClick={() => void commit()}>
              {busy ? <Loader2 size={13} className="animate-spin" /> : <Send size={13} />} Commit ({stagedFiles.length})
            </button>
          </div>
          <details className="ide-git-section" open>
            <summary><ChevronDown size={13} /><span>Changes</span><small>{unstagedFiles.length}</small></summary>
            {unstagedFiles.length ? unstagedFiles.map(file => fileRow(file, false)) : <p className="ide-git-empty">No unstaged changes</p>}
          </details>
          <details className="ide-git-section" open>
            <summary><ChevronDown size={13} /><span>Staged Changes</span><small>{stagedFiles.length}</small></summary>
            {stagedFiles.length ? stagedFiles.map(file => fileRow(file, true)) : <p className="ide-git-empty">Stage files to prepare a commit</p>}
          </details>
          {selected && (
            <div className="ide-git-diff">
              <div className="ide-git-diff-header"><span>{selected.path}</span>{selected.staged && selected.modified && <button type="button" onClick={() => setShowStaged(value => !value)}>{showStaged ? 'Staged' : 'Working tree'} diff</button>}</div>
              {selected.untracked && !selected.staged
                ? <p className="ide-git-empty">Untracked file. Stage it to inspect its diff.</p>
                : diff ? <pre>{diff}{diffTruncated ? '\n… diff truncated at 1 MiB' : ''}</pre> : <p className="ide-git-empty">No diff for this change.</p>}
            </div>
          )}
        </>
      )}
    </section>
  );
};
