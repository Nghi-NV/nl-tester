import React, { useEffect, useMemo, useRef, useState } from 'react';
import { Search } from 'lucide-react';

export interface PaletteCommand {
  id: string;
  label: string;
  detail?: string;
  run: () => void | Promise<void>;
}

interface CommandPaletteProps {
  open: boolean;
  commands: PaletteCommand[];
  onClose: () => void;
}

export const CommandPalette: React.FC<CommandPaletteProps> = ({ open, commands, onClose }) => {
  const [query, setQuery] = useState('');
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const filtered = useMemo(() => {
    const value = query.trim().toLowerCase();
    if (!value) return commands;
    return commands.filter(command => `${command.label} ${command.detail || ''}`.toLowerCase().includes(value));
  }, [commands, query]);

  useEffect(() => {
    if (!open) return;
    setQuery('');
    setSelectedIndex(0);
    requestAnimationFrame(() => inputRef.current?.focus());
  }, [open]);

  useEffect(() => setSelectedIndex(0), [query]);

  if (!open) return null;

  const runSelected = () => {
    const command = filtered[selectedIndex];
    if (!command) return;
    onClose();
    void command.run();
  };

  return (
    <div className="ide-palette-backdrop" onMouseDown={event => event.target === event.currentTarget && onClose()}>
      <section className="ide-command-palette" role="dialog" aria-label="Command Palette">
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
                setSelectedIndex(index => Math.min(index + 1, filtered.length - 1));
              }
              if (event.key === 'ArrowUp') {
                event.preventDefault();
                setSelectedIndex(index => Math.max(index - 1, 0));
              }
              if (event.key === 'Enter') {
                event.preventDefault();
                runSelected();
              }
            }}
            placeholder="Type a command to search"
            aria-label="Search commands"
          />
          <kbd>ESC</kbd>
        </div>
        <div className="ide-palette-results" role="listbox">
          {filtered.length === 0 ? (
            <div className="ide-palette-empty">No matching commands</div>
          ) : filtered.map((command, index) => (
            <button
              key={command.id}
              type="button"
              role="option"
              aria-selected={index === selectedIndex}
              className={`ide-palette-command${index === selectedIndex ? ' is-selected' : ''}`}
              onMouseEnter={() => setSelectedIndex(index)}
              onClick={() => { onClose(); void command.run(); }}
            >
              <span>{command.label}</span>
              {command.detail && <small>{command.detail}</small>}
            </button>
          ))}
        </div>
      </section>
    </div>
  );
};
