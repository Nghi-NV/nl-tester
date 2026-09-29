import React, { useEffect, useMemo, useRef, useState } from 'react';
import { FileCode2, Search } from 'lucide-react';
import { useEditorStore, useFileStore } from '../stores';

interface QuickOpenProps {
  onClose: () => void;
}

const scorePath = (path: string, query: string) => {
  const value = path.toLowerCase();
  const needle = query.toLowerCase();
  const fileName = value.split('/').pop() ?? value;
  if (!needle) return 0;
  const direct = value.indexOf(needle);
  if (direct >= 0) return (fileName.startsWith(needle) ? 1000 : fileName.includes(needle) ? 700 : 500) - direct;

  let score = 0;
  let cursor = 0;
  let consecutive = 0;
  for (const character of needle) {
    const found = value.indexOf(character, cursor);
    if (found < 0) return -1;
    consecutive = found === cursor ? consecutive + 1 : 0;
    score += 1 + consecutive * 2 + (found === 0 || value[found - 1] === '/' || value[found - 1] === '_' || value[found - 1] === '-' ? 5 : 0);
    cursor = found + 1;
  }
  return score;
};

export const QuickOpen: React.FC<QuickOpenProps> = ({ onClose }) => {
  const root = useFileStore(state => state.projectRoot);
  const paths = useFileStore(state => state.workspacePaths);
  const activeFileId = useEditorStore(state => state.activeFileId);
  const openFiles = useEditorStore(state => state.openFiles);
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const recentPaths = useMemo(() => {
    if (!root) return [];
    const normalizedRoot = root.replace(/\\/g, '/').replace(/\/+$/, '');
    const prefix = `${normalizedRoot}/`;
    return [...new Set([activeFileId, ...openFiles.slice().reverse()].filter((path): path is string => !!path).map(path => path.replace(/\\/g, '/')))]
      .filter(path => path.startsWith(prefix))
      .map(path => path.slice(prefix.length));
  }, [activeFileId, openFiles, root]);

  const matches = useMemo(() => {
    const recent = new Set(recentPaths);
    return paths
      .map(path => ({ path, score: query.trim() ? scorePath(path, query.trim()) : recent.has(path) ? 1000 - recentPaths.indexOf(path) : 0, recent: recent.has(path) }))
      .filter(item => item.score >= 0 && (query.trim() ? item.score > 0 : item.recent))
      .sort((left, right) => right.score - left.score || left.path.localeCompare(right.path, undefined, { sensitivity: 'base' }))
      .slice(0, 100);
  }, [paths, query, recentPaths]);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => setSelectedIndex(0), [query]);

  const openSelected = async () => {
    const item = matches[selectedIndex];
    if (!item) return;
    try {
      const filePath = await useFileStore.getState().openWorkspaceFile(item.path);
      useEditorStore.getState().openFile(filePath);
      onClose();
    } catch (error) {
      window.alert(`Could not open ${item.path}: ${String(error)}`);
    }
  };

  return (
    <div className="ide-palette-backdrop" onMouseDown={event => event.target === event.currentTarget && onClose()}>
      <section className="ide-command-palette ide-quick-open" role="dialog" aria-label="Quick Open">
        <div className="ide-palette-input-row">
          <Search size={16} />
          <input
            ref={inputRef}
            value={query}
            onChange={event => setQuery(event.target.value)}
            onKeyDown={event => {
              if (event.key === 'Escape') onClose();
              if (event.key === 'ArrowDown') {
                event.preventDefault();
                setSelectedIndex(index => Math.min(index + 1, matches.length - 1));
              }
              if (event.key === 'ArrowUp') {
                event.preventDefault();
                setSelectedIndex(index => Math.max(index - 1, 0));
              }
              if (event.key === 'Enter') {
                event.preventDefault();
                void openSelected();
              }
            }}
            placeholder={root ? 'Search files by name' : 'Open a folder first'}
            aria-label="Search files by name"
            disabled={!root}
          />
          <kbd>ESC</kbd>
        </div>
        <div className="ide-palette-results" role="listbox" aria-label="Files">
          {!root ? <div className="ide-palette-empty">Open a workspace to find files.</div>
            : !query.trim() && !matches.length ? <div className="ide-palette-empty">Type a filename to search this workspace.</div>
              : !matches.length ? <div className="ide-palette-empty">No matching files.</div>
                : matches.map((item, index) => (
                  <button
                    key={item.path}
                    type="button"
                    role="option"
                    aria-selected={index === selectedIndex}
                    className={`ide-palette-command ide-quick-open-file${index === selectedIndex ? ' is-selected' : ''}`}
                    onMouseEnter={() => setSelectedIndex(index)}
                    onClick={() => void openSelected()}
                  >
                    <FileCode2 size={14} />
                    <span>{item.path.split('/').pop()}</span>
                    <small>{item.path.includes('/') ? item.path.slice(0, item.path.lastIndexOf('/')) : 'workspace root'}</small>
                  </button>
                ))}
        </div>
        <footer className="ide-quick-open-footer">{query.trim() ? `${matches.length} matching files` : 'Recently opened files'}{paths.length > matches.length && query.trim() && ' · narrow your search for more'}</footer>
      </section>
    </div>
  );
};
