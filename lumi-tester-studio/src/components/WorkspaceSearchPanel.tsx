import React, { useCallback, useEffect, useRef, useState } from 'react';
import { CaseSensitive, FileText, Loader2, Regex, Replace, Search, WholeWord, X } from 'lucide-react';
import { useFileStore } from '../stores';
import { replaceWorkspaceText, searchWorkspaceText, WorkspaceSearchOptions, WorkspaceSearchResponse } from '../utils/tauriUtils';

interface WorkspaceSearchPanelProps {
  scopePath: string | null;
  onClearScope: () => void;
}

export const WorkspaceSearchPanel: React.FC<WorkspaceSearchPanelProps> = ({ scopePath, onClearScope }) => {
  const projectRoot = useFileStore(state => state.projectRoot);
  const openSearchResult = useFileStore(state => state.openSearchResult);
  const saveAllFiles = useFileStore(state => state.saveAllFiles);
  const refreshWorkspace = useFileStore(state => state.refresh);
  const dirtyFileIds = useFileStore(state => state.dirtyFileIds);
  const [query, setQuery] = useState('');
  const [replacement, setReplacement] = useState('');
  const [caseSensitive, setCaseSensitive] = useState(false);
  const [wholeWord, setWholeWord] = useState(false);
  const [regex, setRegex] = useState(false);
  const [includePattern, setIncludePattern] = useState('');
  const [excludePattern, setExcludePattern] = useState('');
  const [showReplace, setShowReplace] = useState(false);
  const [showFilters, setShowFilters] = useState(false);
  const [response, setResponse] = useState<WorkspaceSearchResponse | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [isReplacing, setIsReplacing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const sequence = useRef(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const debounceTimer = useRef<number | undefined>(undefined);

  const options: WorkspaceSearchOptions = { caseSensitive, wholeWord, regex, includePattern, excludePattern };

  const search = useCallback(async (text: string) => {
    if (!projectRoot || !text.trim()) return;
    const currentSequence = ++sequence.current;
    setIsLoading(true);
    setError(null);
    try {
      const result = await searchWorkspaceText(projectRoot, text.trim(), scopePath, options);
      if (currentSequence === sequence.current) setResponse(result);
    } catch (searchError) {
      if (currentSequence === sequence.current) {
        setResponse(null);
        setError(String(searchError));
      }
    } finally {
      if (currentSequence === sequence.current) setIsLoading(false);
    }
  }, [projectRoot, scopePath, caseSensitive, wholeWord, regex, includePattern, excludePattern]);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    const trimmed = query.trim();
    if (!trimmed) {
      sequence.current += 1;
      setResponse(null);
      setError(null);
      setIsLoading(false);
      return;
    }
    debounceTimer.current = window.setTimeout(() => void search(trimmed), 250);
    return () => {
      if (debounceTimer.current !== undefined) window.clearTimeout(debounceTimer.current);
      sequence.current += 1;
    };
  }, [query, search]);

  const replaceAll = async () => {
    if (!projectRoot || !query.trim() || !response?.matches.length) return;
    const previewCount = response.truncated ? `at least ${response.matches.length} previewed matches` : `${response.matches.length} matches`;
    if (!window.confirm(`Replace ${previewCount} in matching files${scopePath ? ` under ${scopePath}` : ''}?`)) return;
    setIsReplacing(true);
    setError(null);
    try {
      if (dirtyFileIds.length) await saveAllFiles();
      const result = await replaceWorkspaceText(projectRoot, query.trim(), replacement, scopePath, options);
      await refreshWorkspace();
      setResponse(null);
      setError(null);
      await search(query);
      setReplaceNotice(`${result.replacements} replacement${result.replacements === 1 ? '' : 's'} in ${result.filesChanged} file${result.filesChanged === 1 ? '' : 's'}${result.truncated ? ' (scan limit reached)' : ''}.`);
    } catch (replaceError) {
      setError(String(replaceError));
    } finally {
      setIsReplacing(false);
    }
  };

  const [replaceNotice, setReplaceNotice] = useState<string | null>(null);

  const toggleButton = (label: string, active: boolean, onClick: () => void, icon: React.ReactNode) => (
    <button type="button" className={`ide-search-option${active ? ' is-active' : ''}`} aria-pressed={active} title={label} onClick={onClick}>
      {icon}<span>{label}</span>
    </button>
  );

  return (
    <section className="ide-side-view" aria-label="Workspace Search">
      <header className="ide-side-view-header">
        <span>SEARCH</span>
        <div className="ide-side-view-header-actions">
          <button type="button" title="Toggle replace" aria-label="Toggle replace" aria-pressed={showReplace} onClick={() => setShowReplace(value => !value)}>
            <Replace size={14} />
          </button>
          <button type="button" title="Search filters" aria-label="Search filters" aria-expanded={showFilters} onClick={() => setShowFilters(value => !value)}>
            <span>⋯</span>
          </button>
        {query && (
          <button type="button" title="Clear search" aria-label="Clear search" onClick={() => setQuery('')}>
            <X size={14} />
          </button>
        )}
        </div>
      </header>
      <label className="ide-workspace-search-input">
        <Search size={14} />
        <input
          ref={inputRef}
          value={query}
          onChange={event => setQuery(event.target.value)}
          placeholder="Search in files"
          aria-label="Search workspace contents"
          disabled={!projectRoot}
          onKeyDown={event => {
            if (event.key === 'Enter') {
              if (debounceTimer.current !== undefined) window.clearTimeout(debounceTimer.current);
              void search(query);
            }
            if (event.key === 'Escape') setQuery('');
          }}
        />
        {isLoading && <Loader2 size={14} className="animate-spin" />}
      </label>
      <div className="ide-search-options">
        {toggleButton('Match case', caseSensitive, () => setCaseSensitive(value => !value), <CaseSensitive size={13} />)}
        {toggleButton('Whole word', wholeWord, () => setWholeWord(value => !value), <WholeWord size={13} />)}
        {toggleButton('Regex', regex, () => setRegex(value => !value), <Regex size={13} />)}
      </div>
      {showReplace && (
        <div className="ide-search-replace-row">
          <input className="ide-form-control ide-form-control--compact" value={replacement} onChange={event => setReplacement(event.target.value)} placeholder="Replace with" aria-label="Replace with" />
          <button type="button" className="ide-search-replace-button" disabled={!response?.matches.length || isReplacing} onClick={() => void replaceAll()}>
            {isReplacing ? <Loader2 size={13} className="animate-spin" /> : <Replace size={13} />} Replace all
          </button>
        </div>
      )}
      {showFilters && (
        <div className="ide-search-glob-filters">
          <input className="ide-form-control ide-form-control--compact" value={includePattern} onChange={event => setIncludePattern(event.target.value)} placeholder="Files to include · **/*.yaml" aria-label="Files to include" />
          <input className="ide-form-control ide-form-control--compact" value={excludePattern} onChange={event => setExcludePattern(event.target.value)} placeholder="Files to exclude · **/fixtures/**" aria-label="Files to exclude" />
          <small>Separate multiple glob patterns with commas.</small>
        </div>
      )}
      {replaceNotice && <div className="ide-search-notice" role="status">{replaceNotice}<button type="button" onClick={() => setReplaceNotice(null)} aria-label="Dismiss replace result"><X size={12} /></button></div>}
      {scopePath && (
        <div className="ide-workspace-search-scope" title={`Searching in ${scopePath}`}>
          <span>In folder: {scopePath}</span>
          <button type="button" title="Search entire workspace" aria-label="Search entire workspace" onClick={onClearScope}>
            <X size={13} />
          </button>
        </div>
      )}
      <div className="ide-workspace-search-summary" aria-live="polite">
        {response && <span>{response.matches.length}{response.truncated ? '+' : ''} results · {response.filesScanned} files</span>}
      </div>
      <div className="ide-workspace-search-results">
        {!projectRoot && <p className="ide-view-empty">Open a folder to search its files.</p>}
        {projectRoot && !query.trim() && <p className="ide-view-empty">{scopePath ? `Search text in ${scopePath}.` : 'Search text in workspace files.'} Git-ignored files are skipped.</p>}
        {!!error && <p className="ide-view-error">{error}</p>}
        {response?.matches.map(match => (
          <button
            type="button"
            className="ide-workspace-search-result"
            key={`${match.relativePath}:${match.lineNumber}:${match.column}`}
            title={`${match.relativePath}:${match.lineNumber}`}
            onClick={() => void openSearchResult(match.relativePath, match.lineNumber, match.column).catch(openError => setError(String(openError)))}
          >
            <span className="ide-workspace-search-file">
              <FileText size={13} />
              <span>{match.relativePath}</span>
              <small>{match.lineNumber}</small>
            </span>
            <code>{match.line}</code>
          </button>
        ))}
        {response && response.matches.length === 0 && <p className="ide-view-empty">No results found.</p>}
        {response?.truncated && <p className="ide-view-empty">More results may be available; search reached its limit.</p>}
      </div>
      {projectRoot && <footer className="ide-side-view-footer"><Search size={12} /><span>Hidden files included · gitignore honored</span></footer>}
    </section>
  );
};
