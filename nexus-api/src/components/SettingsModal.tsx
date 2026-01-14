import React, { useState } from 'react';
import { useEnvStore } from '../stores';
import { X, Plus, Trash2, Save } from 'lucide-react';

interface Props {
  onClose: () => void;
}

export const SettingsModal: React.FC<Props> = ({ onClose }) => {
  const { envVars, setEnvVars } = useEnvStore();
  const [localVars, setLocalVars] = useState(envVars);

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
          <h2 className="text-lg font-bold text-white">Environment Variables</h2>
          <button onClick={onClose} className="text-slate-400 hover:text-white transition-colors">
            <X size={20} />
          </button>
        </div>

        <div className="p-6 overflow-y-auto flex-1">
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
                  className="bg-slate-950 border border-slate-700 rounded px-3 py-2 text-sm text-white focus:border-cyan-500 outline-none flex-1"
                />
                <div className="text-slate-600">=</div>
                <input
                  type="text"
                  placeholder="Value"
                  value={v.value}
                  onChange={(e) => updateVar(idx, 'value', e.target.value)}
                  className="bg-slate-950 border border-slate-700 rounded px-3 py-2 text-sm text-cyan-300 focus:border-cyan-500 outline-none flex-1 font-mono"
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
        </div>

        <div className="p-6 border-t border-borderGlass bg-slate-900/50 flex justify-end gap-3 rounded-b-xl">
          <button
            onClick={onClose}
            className="px-4 py-2 text-sm font-medium text-slate-400 hover:text-white transition-colors"
          >
            Cancel
          </button>
          <button
            onClick={handleSave}
            className="px-6 py-2 bg-cyan-600 hover:bg-cyan-500 text-white text-sm font-medium rounded-lg shadow-lg hover:shadow-cyan-500/20 transition-all flex items-center gap-2"
          >
            <Save size={16} /> Save Changes
          </button>
        </div>
      </div>
    </div>
  );
};