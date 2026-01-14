import React from 'react';
import { AlertTriangle, X } from 'lucide-react';
import { clsx } from 'clsx';

interface ConfirmModalProps {
  isOpen: boolean;
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  isDangerous?: boolean;
  onConfirm: () => void;
  onCancel: () => void;
}

export const ConfirmModal: React.FC<ConfirmModalProps> = ({
  isOpen,
  title,
  message,
  confirmLabel = 'Confirm',
  cancelLabel = 'Cancel',
  isDangerous = false,
  onConfirm,
  onCancel
}) => {
  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-[60] flex items-center justify-center bg-black/60 backdrop-blur-sm animate-in fade-in duration-200">
      <div className="w-[400px] bg-slate-950 border border-slate-700 rounded-xl shadow-2xl overflow-hidden animate-in zoom-in-95 duration-200">

        {/* Header */}
        <div className="p-4 border-b border-white/5 bg-slate-900/50 backdrop-blur flex justify-between items-center">
          <h3 className="text-lg font-bold text-slate-100 flex items-center gap-2">
            {isDangerous && <AlertTriangle size={20} className="text-rose-500" />}
            {title}
          </h3>
          <button onClick={onCancel} className="text-slate-500 hover:text-white transition-colors">
            <X size={20} />
          </button>
        </div>

        {/* Content */}
        <div className="p-6">
          <p className="text-slate-300 text-sm leading-relaxed">{message}</p>
        </div>

        {/* Actions */}
        <div className="p-4 bg-slate-900/30 flex justify-end gap-3 border-t border-white/5">
          <button
            onClick={onCancel}
            className="px-4 py-2 rounded-lg text-sm font-medium text-slate-400 hover:bg-slate-800 transition-colors"
          >
            {cancelLabel}
          </button>
          <button
            onClick={onConfirm}
            className={clsx(
              "px-4 py-2 rounded-lg text-sm font-bold text-white transition-all shadow-lg",
              isDangerous
                ? "bg-rose-600 hover:bg-rose-500 shadow-rose-900/20"
                : "bg-cyan-600 hover:bg-cyan-500 shadow-cyan-900/20"
            )}
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
};
