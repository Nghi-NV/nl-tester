import React from 'react';
import { BarChart3, Bot, Files, ListChecks, ScanSearch, Puzzle, Search, Settings } from 'lucide-react';
import { clsx } from 'clsx';

export type ActivityId = 'explorer' | 'search' | 'tests' | 'inspector' | 'extensions' | 'reports' | 'ai';

interface ActivityBarProps {
  active: ActivityId | null;
  onSelect: (activity: ActivityId | 'settings') => void;
}

const activities: Array<{ id: ActivityId; label: string; icon: React.ReactNode }> = [
  { id: 'explorer', label: 'Explorer', icon: <Files size={22} strokeWidth={1.6} /> },
  { id: 'search', label: 'Search', icon: <Search size={21} strokeWidth={1.6} /> },
  { id: 'tests', label: 'Test Explorer', icon: <ListChecks size={21} strokeWidth={1.6} /> },
  { id: 'inspector', label: 'UI Inspector', icon: <ScanSearch size={21} strokeWidth={1.6} /> },
  { id: 'extensions', label: 'Lumi Extensions', icon: <Puzzle size={21} strokeWidth={1.6} /> },
  { id: 'reports', label: 'Test Reports', icon: <BarChart3 size={21} strokeWidth={1.6} /> },
  { id: 'ai', label: 'AI Assistant', icon: <Bot size={21} strokeWidth={1.6} /> },
];

export const ActivityBar: React.FC<ActivityBarProps> = ({ active, onSelect }) => (
  <nav className="ide-activity-bar" aria-label="Primary navigation">
    <div className="ide-activity-group">
      {activities.map(item => (
        <button
          key={item.id}
          type="button"
          aria-label={item.label}
          aria-pressed={active === item.id}
          title={item.label}
          onClick={() => onSelect(item.id)}
          className={clsx('ide-activity-button', active === item.id && 'is-active')}
        >
          {item.icon}
        </button>
      ))}
    </div>
    <button
      type="button"
      aria-label="Settings"
      title="Settings"
      onClick={() => onSelect('settings')}
      className="ide-activity-button"
    >
      <Settings size={21} strokeWidth={1.6} />
    </button>
  </nav>
);
