import React, { useCallback, useEffect, useRef, useState } from 'react';
import { FileText, Loader2, Search, X } from 'lucide-react';
import { useFileStore } from '../stores';
import { WorkspaceSearchResponse, searchWorkspaceText } from '../utils/tauriUtils';

export const WorkspaceSearchPanel: React.FC = () => {
  const projectRoot = useFileStore(state => state.projectRoot);
  const openSearchResult = useFileStore(state => state.openSearchResult);
  const [query, setQuery] = useState('');
  const [response, setResponse] = useState<WorkspaceSearchResponse | null>(null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const sequence = useRef(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const debounceTimer = useRef<number | undefined>(undefined);

  const search = useCallback(async (text: string) => {
    if (!projectRoot || !text.trim()) return;
    const currentSequence = ++sequence.current;
    setIsLoading(true);
    setError(null);
    try {
      const result = await searchWorkspaceText(projectRoot, text.trim());
      if (currentSequence === sequence.current) setResponse(result);
    } catch (searchError) {
      if (currentSequence === sequence.current) {
        setResponse(null);
        setError(String(searchError));
      }
    } finally {
      if (currentSequence === sequence.current) setIsLoading(false);
    }
  }, [projectRoot]);

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

  return (
    <section className="ide-side-view" aria-label="Workspace Search">
      <header className="ide-side-view-header">
        <span>SEARCH</span>
        {query && (
          <button type="button" title="Clear search" aria-label="Clear search" onClick={() => setQuery('')}>
            <X size={14} />
          </button>
        )}
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
      <div className="ide-workspace-search-summary" aria-live="polite">
        {response && <span>{response.matches.length}{response.truncated ? '+' : ''} results · {response.filesScanned} files</span>}
      </div>
      <div className="ide-workspace-search-results">
        {!projectRoot && <p className="ide-view-empty">Open a folder to search its files.</p>}
        {projectRoot && !query.trim() && <p className="ide-view-empty">Search text in workspace files. Git-ignored files are skipped.</p>}
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
