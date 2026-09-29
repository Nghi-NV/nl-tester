import React, { useEffect, useRef, useState } from 'react';
import { useDeviceStore } from '../stores';
import { Check, ChevronDown, CircleAlert, Monitor, RefreshCw, Smartphone } from 'lucide-react';
import { clsx } from 'clsx';

type Platform = 'android' | 'ios' | 'web';

const platformLabel: Record<Platform, string> = {
  android: 'Android',
  ios: 'iOS',
  web: 'Web',
};

const platformIcon = (platform: Platform) => platform === 'web'
  ? <Monitor size={14} />
  : <Smartphone size={14} className={platform === 'android' ? 'is-android' : 'is-ios'} />;

export const DeviceSelector: React.FC = () => {
  const {
    allDevices,
    selectedDevice,
    selectedPlatform,
    isLoading,
    refreshAllDevices,
    setSelectedDevice,
    setSelectedPlatform,
  } = useDeviceStore();
  const [isOpen, setIsOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    void refreshAllDevices();
  }, [refreshAllDevices]);

  useEffect(() => {
    const openDeviceSelector = () => {
      setIsOpen(true);
      void refreshAllDevices();
    };
    window.addEventListener('lumi-open-device-selector', openDeviceSelector);
    return () => window.removeEventListener('lumi-open-device-selector', openDeviceSelector);
  }, [refreshAllDevices]);

  useEffect(() => {
    if (!isOpen) return;
    const closeOnOutsidePointer = (event: PointerEvent) => {
      if (event.target instanceof Node && !rootRef.current?.contains(event.target)) setIsOpen(false);
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setIsOpen(false);
    };
    document.addEventListener('pointerdown', closeOnOutsidePointer);
    document.addEventListener('keydown', closeOnEscape);
    return () => {
      document.removeEventListener('pointerdown', closeOnOutsidePointer);
      document.removeEventListener('keydown', closeOnEscape);
    };
  }, [isOpen]);

  const devicesFor = (platform: Exclude<Platform, 'web'>) => allDevices
    .filter(device => device.platform === platform)
    .sort((left, right) => left.name.localeCompare(right.name));
  const activeDevice = allDevices.find(device => device.id === selectedDevice && device.platform === selectedPlatform);
  const selectedLabel = selectedPlatform === 'web'
    ? 'Web Browser'
    : activeDevice?.name.replace(/\s*\([^)]+\)$/, '') || selectedDevice || 'No Device Selected';

  const select = (platform: Platform, deviceId: string | null) => {
    setSelectedPlatform(platform);
    setSelectedDevice(deviceId);
    setIsOpen(false);
  };

  const renderPlatform = (platform: Platform) => {
    const devices = platform === 'web' ? [] : devicesFor(platform);
    const isPlatformSelected = selectedPlatform === platform && !selectedDevice;
    return (
      <section className="ide-device-group" key={platform} aria-label={`${platformLabel[platform]} devices`}>
        <div className="ide-device-group-heading">
          <span>{platformIcon(platform)}{platformLabel[platform]}</span>
          <button
            type="button"
            className={clsx('ide-device-platform-action', isPlatformSelected && 'is-selected')}
            onClick={() => select(platform, null)}
          >
            {isPlatformSelected && <Check size={12} />}
            Use platform
          </button>
        </div>
        {platform === 'web' ? (
          <button
            type="button"
            role="option"
            aria-selected={selectedPlatform === 'web'}
            className={clsx('ide-device-option', selectedPlatform === 'web' && 'is-selected')}
            onClick={() => select('web', null)}
          >
            <span className="ide-device-option-icon">{platformIcon('web')}</span>
            <span className="ide-device-option-copy"><strong>Web Browser</strong><small>Run with the local browser</small></span>
            {selectedPlatform === 'web' && <Check size={15} className="ide-device-check" />}
          </button>
        ) : devices.length > 0 ? devices.map(device => {
          const isSelected = selectedPlatform === platform && selectedDevice === device.id;
          return (
            <button
              type="button"
              role="option"
              aria-selected={isSelected}
              className={clsx('ide-device-option', isSelected && 'is-selected')}
              key={device.id}
              onClick={() => select(platform, device.id)}
            >
              <span className="ide-device-option-icon">{platformIcon(platform)}</span>
              <span className="ide-device-option-copy">
                <strong>{device.name.replace(/\s*\([^)]+\)$/, '')}</strong>
                <small>{device.id}</small>
              </span>
              {isSelected && <Check size={15} className="ide-device-check" />}
            </button>
          );
        }) : (
          <div className="ide-device-empty">
            <CircleAlert size={13} />
            {platform === 'android' ? 'No Android devices found. Connect a device or start an emulator.' : 'No iOS simulators or devices found.'}
          </div>
        )}
      </section>
    );
  };

  return (
    <div className="ide-device-selector" ref={rootRef}>
      <button
        type="button"
        className="ide-device-trigger"
        title="Select device"
        aria-label={`Selected device: ${selectedLabel}`}
        aria-haspopup="listbox"
        aria-expanded={isOpen}
        onClick={() => {
          setIsOpen(open => !open);
          if (!isOpen) void refreshAllDevices();
        }}
      >
        <span className={clsx('ide-device-status-dot', selectedPlatform === 'web' || activeDevice ? 'is-connected' : 'is-idle')} />
        {platformIcon(selectedPlatform)}
        <span className="ide-device-trigger-label">{selectedLabel}</span>
        <ChevronDown size={12} />
      </button>
      {isOpen && (
        <div className="ide-device-menu" role="listbox" aria-label="Select a device">
          <div className="ide-device-menu-header">
            <strong>Select Device</strong>
            <button type="button" onClick={() => void refreshAllDevices()} disabled={isLoading} title="Refresh devices">
              <RefreshCw size={13} className={clsx(isLoading && 'is-spinning')} />
              Refresh
            </button>
          </div>
          <div className="ide-device-menu-content">
            {renderPlatform('android')}
            {renderPlatform('ios')}
            {renderPlatform('web')}
          </div>
          <div className="ide-device-menu-footer">
            {selectedDevice && !activeDevice && selectedPlatform !== 'web'
              ? 'Previously selected device is not connected.'
              : 'Device selection is used when running a test.'}
          </div>
        </div>
      )}
    </div>
  );
};
