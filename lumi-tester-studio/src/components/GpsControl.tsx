import React, { useEffect, useMemo, useRef, useState } from 'react';
import { createPortal } from 'react-dom';
import { invoke } from '@tauri-apps/api/core';
import { Activity, ChevronDown, Compass, MapPin, Pause, Play, X } from 'lucide-react';
import { findFile, useDeviceStore, useEditorStore, useExecutionStore, useFileStore } from '../stores';

type SpeedMode = 'linear' | 'noise';

interface GpsSettings {
  route?: string;
  speed: number;
  speedMode: SpeedMode;
}

function readGpsSettings(content: string): GpsSettings | null {
  const command = /^\s*-\s*(?:mockLocation|gps)\b\s*:?[ \t]*(.*)$/im.exec(content);
  if (!command || command.index === undefined) return null;

  const commandBody = content.slice(command.index + command[0].length)
    .split(/^\s*-\s+\S/m, 1)[0];
  const source = `${command[1] ?? ''}\n${commandBody}`;
  const routeMatch = /(?:^|[,{\s])(?:file|path|route)\s*:\s*(?:"([^"]+)"|'([^']+)'|([^,}\s#]+))|^\s*(?:"([^"]+\.gpx|[^"]+\.kml|[^"]+\.json)"|'([^']+\.gpx|[^']+\.kml|[^']+\.json)'|([^\s#]+\.(?:gpx|kml|json)))/im.exec(source);
  const speedMatch = /(?:^|[,{\s])speed\s*:\s*(\d+(?:\.\d+)?)/i.exec(source);
  const modeMatch = /(?:^|[,{\s])speedMode\s*:\s*(?:"(linear|noise)"|'(linear|noise)'|(linear|noise)\b)/i.exec(source);

  return {
    route: routeMatch?.slice(1).find(Boolean)?.trim(),
    speed: speedMatch ? Math.max(0, Math.min(200, Math.round(Number(speedMatch[1])))) : 60,
    speedMode: (modeMatch?.slice(1).find(Boolean)?.toLowerCase() ?? 'linear') as SpeedMode,
  };
}

export const GpsControl: React.FC = () => {
  const files = useFileStore(state => state.files);
  const activeFileId = useEditorStore(state => state.activeFileId);
  const isRunning = useExecutionStore(state => state.isRunning);
  const selectedPlatform = useDeviceStore(state => state.selectedPlatform);
  const activeFile = activeFileId ? findFile(files, activeFileId) : undefined;
  const isYaml = Boolean(activeFile?.type === 'file' && /\.ya?ml$/i.test(activeFile.name));
  const gpsSettings = useMemo(
    () => isYaml && activeFile?.content !== undefined ? readGpsSettings(activeFile.content) : null,
    [activeFile?.content, isYaml],
  );
  const [isOpen, setIsOpen] = useState(false);
  const [speed, setSpeed] = useState(gpsSettings?.speed ?? 60);
  const [speedMode, setSpeedMode] = useState<SpeedMode>(gpsSettings?.speedMode ?? 'linear');
  const [isPaused, setIsPaused] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const previousRunState = useRef(false);
  const speedUpdateTimer = useRef<number | null>(null);
  const canControl = isRunning && (selectedPlatform === 'android' || selectedPlatform === 'ios');

  useEffect(() => {
    if (speedUpdateTimer.current !== null) {
      window.clearTimeout(speedUpdateTimer.current);
      speedUpdateTimer.current = null;
    }
    setSpeed(gpsSettings?.speed ?? 60);
    setSpeedMode(gpsSettings?.speedMode ?? 'linear');
    setIsPaused(false);
    setError(null);
  }, [activeFileId, gpsSettings?.route, gpsSettings?.speed, gpsSettings?.speedMode]);

  useEffect(() => {
    if (isRunning && !previousRunState.current && gpsSettings) setIsOpen(true);
    if (!isRunning) {
      setIsPaused(false);
      if (speedUpdateTimer.current !== null) {
        window.clearTimeout(speedUpdateTimer.current);
        speedUpdateTimer.current = null;
      }
    }
    previousRunState.current = isRunning;
  }, [isRunning, gpsSettings]);

  useEffect(() => () => {
    if (speedUpdateTimer.current !== null) window.clearTimeout(speedUpdateTimer.current);
  }, []);

  const sendControl = async (control: { speed?: number; paused?: boolean; speedMode?: SpeedMode }) => {
    setError(null);
    try {
      await invoke('set_gps_control', control);
      return true;
    } catch (invokeError) {
      setError(String(invokeError));
      return false;
    }
  };

  const changeSpeed = (value: number) => {
    const nextSpeed = Math.max(0, Math.min(200, Math.round(value)));
    setSpeed(nextSpeed);
    if (!canControl) return;
    if (speedUpdateTimer.current !== null) window.clearTimeout(speedUpdateTimer.current);
    speedUpdateTimer.current = window.setTimeout(() => {
      void sendControl({ speed: nextSpeed });
      speedUpdateTimer.current = null;
    }, 140);
  };

  const changeMode = async (mode: SpeedMode) => {
    const previousMode = speedMode;
    setSpeedMode(mode);
    if (!await sendControl({ speedMode: mode })) setSpeedMode(previousMode);
  };

  const togglePause = async () => {
    const nextPaused = !isPaused;
    setIsPaused(nextPaused);
    if (!await sendControl({ paused: nextPaused })) setIsPaused(!nextPaused);
  };

  if (!gpsSettings) return null;

  return (
    <>
      <button
        className={`ide-status-item ide-gps-trigger${isOpen ? ' is-active' : ''}`}
        type="button"
        title="Open live GPS route controls"
        aria-label="Open live GPS route controls"
        aria-expanded={isOpen}
        onClick={() => setIsOpen(open => !open)}
      >
        <Compass size={13} /> GPS control <ChevronDown size={11} className={isOpen ? 'is-expanded' : ''} />
      </button>
      {isOpen && createPortal(
        <>
          <button className="ide-gps-dismiss-layer" type="button" aria-label="Close GPS controls" onClick={() => setIsOpen(false)} />
          <section className="ide-gps-popover" role="dialog" aria-label="Live GPS route controls">
            <header className="ide-gps-header">
              <span className="ide-gps-header-icon"><Compass size={15} /></span>
              <div className="ide-gps-header-copy">
                <strong>GPS route controls</strong>
                <span>{activeFile?.name}</span>
              </div>
              <span className={`ide-gps-run-state${isRunning ? ' is-live' : ''}`}><i />{isRunning ? 'Live' : 'Idle'}</span>
              <button className="ide-gps-close" type="button" title="Close GPS controls" aria-label="Close GPS controls" onClick={() => setIsOpen(false)}><X size={14} /></button>
            </header>

            <div className="ide-gps-route" title={gpsSettings.route ?? 'Location route is configured in this YAML flow'}>
              <MapPin size={13} />
              <span>{gpsSettings.route ?? 'Location route from active YAML flow'}</span>
            </div>

            <div className="ide-gps-speed-card">
              <div className="ide-gps-speed-value"><strong>{speed}</strong><span>km/h</span></div>
              <label htmlFor="ide-gps-speed-slider">Playback speed</label>
              <input
                id="ide-gps-speed-slider"
                type="range"
                min="0"
                max="200"
                step="1"
                value={speed}
                disabled={!canControl}
                onChange={event => changeSpeed(Number(event.currentTarget.value))}
                aria-valuetext={`${speed} kilometers per hour`}
              />
              <div className="ide-gps-speed-scale"><span>0</span><span>200 km/h</span></div>
            </div>

            <div className="ide-gps-presets" aria-label="Speed presets">
              {[
                { label: 'Walk', speed: 5 },
                { label: 'Cycle', speed: 20 },
                { label: 'Drive', speed: 60 },
                { label: 'Fast', speed: 120 },
              ].map(preset => (
                <button type="button" key={preset.label} className={speed === preset.speed ? 'is-selected' : ''} disabled={!canControl} onClick={() => changeSpeed(preset.speed)}>
                  <span>{preset.label}</span><strong>{preset.speed}</strong>
                </button>
              ))}
            </div>

            <div className="ide-gps-control-row">
              <div className="ide-gps-mode" role="group" aria-label="Speed mode">
                <button type="button" className={speedMode === 'linear' ? 'is-selected' : ''} disabled={!canControl} onClick={() => void changeMode('linear')}><Activity size={12} /> Linear</button>
                <button type="button" className={speedMode === 'noise' ? 'is-selected' : ''} disabled={!canControl} onClick={() => void changeMode('noise')}>Noise</button>
              </div>
              <button className={`ide-gps-pause${isPaused ? ' is-paused' : ''}`} type="button" disabled={!canControl} onClick={() => void togglePause()}>
                {isPaused ? <Play size={13} /> : <Pause size={13} />}{isPaused ? 'Resume' : 'Pause'}
              </button>
            </div>

            {error && <p className="ide-gps-error" role="alert">{error}</p>}
            {!isRunning && <p className="ide-gps-hint">Run this flow to enable live speed and pause controls. Location comes from the route file.</p>}
            {isRunning && !canControl && <p className="ide-gps-hint">Live GPS playback controls are available for Android and iOS runs.</p>}
          </section>
        </>,
        document.body,
      )}
    </>
  );
};
