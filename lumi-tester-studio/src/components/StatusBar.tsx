import React from 'react';
import { useDeviceStore, useEditorStore, useExecutionStore, useFileStore } from '../stores';
import { CheckCircle2, CircleAlert, Clock3, Loader2 } from 'lucide-react';

export const StatusBar: React.FC = () => {
  const dirtyCount = useFileStore(state => state.dirtyFileIds.length);
  const fileValidation = useFileStore(state => state.fileValidation);
  const activeFileId = useEditorStore(state => state.activeFileId);
  const selectedPlatform = useDeviceStore(state => state.selectedPlatform);
  const selectedDevice = useDeviceStore(state => state.selectedDevice);
  const { isRunning, currentRunLabel, queuedRuns } = useExecutionStore();
  const validation = activeFileId && /\.ya?ml$/i.test(activeFileId)
    ? fileValidation[activeFileId]
    : undefined;

  return (
    <footer className="ide-status-bar" aria-label="Workspace status">
      <div className="ide-status-left">
        {dirtyCount > 0 && <span className="ide-status-item" title="Changes are saved automatically after editing pauses">Auto-save pending</span>}
        {isRunning && <span className="ide-status-item" title={currentRunLabel ?? undefined}><Loader2 size={13} className="animate-spin" /> Running test</span>}
        {queuedRuns.length > 0 && <span className="ide-status-item"><Clock3 size={13} /> {queuedRuns.length} queued</span>}
      </div>
      <div className="ide-status-right">
        {validation && (
          <span
            className={`ide-status-item ide-validation-status is-${validation.state}`}
            title={validation.message ?? (validation.state === 'valid' ? 'YAML is valid' : validation.state === 'checking' ? 'Checking YAML' : validation.state === 'invalid' ? 'YAML validation failed' : 'File operation failed')}
          >
            {validation.state === 'checking'
              ? <><Loader2 size={13} className="animate-spin" /> Checking</>
              : validation.state === 'valid'
                ? <><CheckCircle2 size={13} /> Valid</>
                : validation.state === 'invalid'
                  ? <><CircleAlert size={13} /> Invalid</>
                  : <><CircleAlert size={13} /> Error</>}
          </span>
        )}
        <span className="ide-status-item">Lumi YAML</span>
        <span className="ide-status-item">{selectedPlatform}</span>
        <span className="ide-status-item">{selectedPlatform === 'web' ? 'Browser' : selectedDevice || 'No device selected'}</span>
      </div>
    </footer>
  );
};
