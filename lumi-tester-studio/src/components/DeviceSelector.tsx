import React, { useEffect, useState } from 'react';
import { useDeviceStore } from '../stores';
import { Smartphone, Monitor, RefreshCw, ChevronDown, X, Check } from 'lucide-react';
import { clsx } from 'clsx';

export const DeviceSelector: React.FC = () => {
  const {
    allDevices,
    selectedDevice,
    selectedPlatform,
    isLoading,
    refreshAllDevices,
    setSelectedDevice,
    setSelectedPlatform
  } = useDeviceStore();

  const [isModalOpen, setIsModalOpen] = useState(false);

  useEffect(() => {
    refreshAllDevices();
    // Poll for devices every 5 seconds
    const interval = setInterval(refreshAllDevices, 5000);
    return () => clearInterval(interval);
  }, [refreshAllDevices]);

  const getPlatformIcon = (platform: string) => {
    switch (platform) {
      case 'android':
        return <Smartphone size={14} className="text-green-500" />;
      case 'ios':
        return <Smartphone size={14} className="text-blue-500" />;
      case 'web':
        return <Monitor size={14} />;
      default:
        return <Smartphone size={14} />;
    }
  };

  // Extract device ID from name (format: "Name (ID)" or just "ID")
  const extractDeviceId = (name: string, id: string): string => {
    // If name contains ID in parentheses, extract it
    const match = name.match(/\(([^)]+)\)$/);
    if (match) {
      return match[1];
    }
    // Otherwise return the id
    return id;
  };

  // Extract device name without ID (format: "Name (ID)" -> "Name")
  const extractDeviceName = (name: string, id: string): string => {
    // If name contains ID in parentheses, remove it
    const match = name.match(/^(.+?)\s*\([^)]+\)$/);
    if (match) {
      return match[1].trim();
    }
    // If name is just the ID, return a friendly message
    if (name === id) {
      return 'Device';
    }
    return name;
  };

  const getDisplayText = () => {
    if (selectedPlatform === 'web') {
      return 'Web';
    }
    if (selectedDevice) {
      const device = allDevices.find(d => d.id === selectedDevice && d.platform === selectedPlatform);
      if (device) {
        return extractDeviceName(device.name, device.id);
      }
      return selectedDevice;
    }
    return 'Select Device';
  };

  const handleDeviceSelect = (deviceId: string | null, platform: 'android' | 'ios' | 'web') => {
    setSelectedPlatform(platform);
    setSelectedDevice(deviceId);
    setIsModalOpen(false);
  };

  // Get device priority for sorting (lower number = higher priority)
  const getDevicePriority = (deviceName: string): number => {
    const name = deviceName.toLowerCase();
    if (name.includes('iphone')) return 1;
    if (name.includes('ipad')) return 2;
    if (name.includes('apple tv') || name.includes('appletv')) return 3;
    if (name.includes('watch')) return 4;
    return 5; // Other devices
  };

  // Group devices by platform and sort iOS devices by priority
  const devicesByPlatform = {
    android: allDevices.filter(d => d.platform === 'android'),
    ios: allDevices
      .filter(d => d.platform === 'ios')
      .sort((a, b) => {
        const priorityA = getDevicePriority(a.name);
        const priorityB = getDevicePriority(b.name);
        if (priorityA !== priorityB) {
          return priorityA - priorityB;
        }
        // If same priority, sort alphabetically
        return a.name.localeCompare(b.name);
      }),
    web: [] as typeof allDevices
  };

  return (
    <>
      <div className="flex items-center gap-2 mr-2">
        <button
          className="flex items-center gap-1 px-2 py-1.5 rounded bg-slate-800 text-slate-300 hover:text-cyan-400 hover:bg-slate-700 border border-transparent transition-all text-xs font-medium min-w-[140px] justify-between"
          title="Select Device"
          onClick={() => {
            setIsModalOpen(true);
            refreshAllDevices();
          }}
        >
          <div className="flex items-center gap-1 truncate max-w-[120px]">
            {getPlatformIcon(selectedPlatform)}
            <span className="truncate">{getDisplayText()}</span>
          </div>
          <ChevronDown size={12} />
        </button>
      </div>

      {/* Device Selection Modal */}
      {isModalOpen && (
        <div
          className="fixed inset-0 z-[60] flex items-center justify-center bg-black/60 backdrop-blur-sm animate-in fade-in duration-200"
          onClick={() => setIsModalOpen(false)}
        >
          <div
            className="w-[500px] max-h-[80vh] bg-slate-950 border border-slate-700 rounded-xl shadow-2xl overflow-hidden animate-in zoom-in-95 duration-200 flex flex-col"
            onClick={(e) => e.stopPropagation()}
          >
            {/* Header */}
            <div className="p-4 border-b border-slate-700 bg-slate-900/50 backdrop-blur flex justify-between items-center">
              <h3 className="text-lg font-bold text-slate-100 flex items-center gap-2">
                <Smartphone size={20} />
                Select Device
              </h3>
              <button
                onClick={() => setIsModalOpen(false)}
                className="text-slate-500 hover:text-white transition-colors"
              >
                <X size={20} />
              </button>
            </div>

            {/* Content */}
            <div className="flex-1 overflow-y-auto p-4">
              {/* Platform: Android */}
              <div className="mb-6">
                <div className="flex items-center justify-between mb-2">
                  <div className="text-sm font-semibold text-slate-300 flex items-center gap-2">
                    <Smartphone size={16} className="text-green-500" />
                    Android
                  </div>
                  <button
                    onClick={() => handleDeviceSelect(null, 'android')}
                    className={clsx(
                      "px-3 py-1 text-xs rounded transition-colors flex items-center gap-1.5",
                      selectedPlatform === 'android' && !selectedDevice
                        ? "bg-cyan-600 text-white"
                        : "bg-slate-800 text-slate-400 hover:bg-slate-700"
                    )}
                  >
                    {selectedPlatform === 'android' && !selectedDevice && (
                      <Check size={12} />
                    )}
                    Select Platform
                  </button>
                </div>
                {devicesByPlatform.android.length === 0 ? (
                  <div className="text-xs text-slate-500 italic pl-6">
                    No Android devices found. Check USB debugging.
                  </div>
                ) : (
                  <div className="space-y-1">
                    {devicesByPlatform.android.map(device => {
                      const deviceName = extractDeviceName(device.name, device.id);
                      const deviceId = extractDeviceId(device.name, device.id);
                      return (
                        <button
                          key={device.id}
                          onClick={() => handleDeviceSelect(device.id, 'android')}
                          className={clsx(
                            "w-full text-left px-4 py-3 rounded transition-colors flex items-start gap-3",
                            selectedDevice === device.id && selectedPlatform === 'android'
                              ? "bg-cyan-600/20 text-cyan-400 border border-cyan-600/30"
                              : "bg-slate-800 text-slate-300 hover:bg-slate-700"
                          )}
                        >
                          <Smartphone size={16} className="text-green-500 mt-0.5 flex-shrink-0" />
                          <div className="flex-1 min-w-0">
                            <div className="font-medium truncate">{deviceName}</div>
                            <div className="text-xs text-slate-500 truncate mt-0.5">{deviceId}</div>
                          </div>
                          {selectedDevice === device.id && selectedPlatform === 'android' && (
                            <Check size={18} className="text-cyan-400 flex-shrink-0 mt-0.5" />
                          )}
                        </button>
                      );
                    })}
                  </div>
                )}
              </div>

              {/* Platform: iOS */}
              <div className="mb-6">
                <div className="flex items-center justify-between mb-2">
                  <div className="text-sm font-semibold text-slate-300 flex items-center gap-2">
                    <Smartphone size={16} className="text-blue-500" />
                    iOS
                  </div>
                  <button
                    onClick={() => handleDeviceSelect(null, 'ios')}
                    className={clsx(
                      "px-3 py-1 text-xs rounded transition-colors flex items-center gap-1.5",
                      selectedPlatform === 'ios' && !selectedDevice
                        ? "bg-cyan-600 text-white"
                        : "bg-slate-800 text-slate-400 hover:bg-slate-700"
                    )}
                  >
                    {selectedPlatform === 'ios' && !selectedDevice && (
                      <Check size={12} />
                    )}
                    Select Platform
                  </button>
                </div>
                {devicesByPlatform.ios.length === 0 ? (
                  <div className="text-xs text-slate-500 italic pl-6">
                    No iOS devices found. Check idb connection.
                  </div>
                ) : (
                  <div className="space-y-1">
                    {devicesByPlatform.ios.map(device => {
                      const deviceName = extractDeviceName(device.name, device.id);
                      const deviceId = extractDeviceId(device.name, device.id);
                      return (
                        <button
                          key={device.id}
                          onClick={() => handleDeviceSelect(device.id, 'ios')}
                          className={clsx(
                            "w-full text-left px-4 py-3 rounded transition-colors flex items-start gap-3",
                            selectedDevice === device.id && selectedPlatform === 'ios'
                              ? "bg-cyan-600/20 text-cyan-400 border border-cyan-600/30"
                              : "bg-slate-800 text-slate-300 hover:bg-slate-700"
                          )}
                        >
                          <Smartphone size={16} className="text-blue-500 mt-0.5 flex-shrink-0" />
                          <div className="flex-1 min-w-0">
                            <div className="font-medium truncate">{deviceName}</div>
                            <div className="text-xs text-slate-500 truncate mt-0.5">{deviceId}</div>
                          </div>
                          {selectedDevice === device.id && selectedPlatform === 'ios' && (
                            <Check size={18} className="text-cyan-400 flex-shrink-0 mt-0.5" />
                          )}
                        </button>
                      );
                    })}
                  </div>
                )}
              </div>

              {/* Platform: Web */}
              <div>
                <div className="flex items-center justify-between mb-2">
                  <div className="text-sm font-semibold text-slate-300 flex items-center gap-2">
                    <Monitor size={16} />
                    Web
                  </div>
                  <button
                    onClick={() => handleDeviceSelect(null, 'web')}
                    className={clsx(
                      "px-3 py-1 text-xs rounded transition-colors flex items-center gap-1.5",
                      selectedPlatform === 'web'
                        ? "bg-cyan-600 text-white"
                        : "bg-slate-800 text-slate-400 hover:bg-slate-700"
                    )}
                  >
                    {selectedPlatform === 'web' && (
                      <Check size={12} />
                    )}
                    Select Platform
                  </button>
                </div>
                <div className="text-xs text-slate-500 italic pl-6">
                  Web browser automation
                </div>
              </div>
            </div>

            {/* Footer */}
            <div className="p-4 bg-slate-900/30 flex justify-between items-center border-t border-slate-700">
              <button
                onClick={refreshAllDevices}
                disabled={isLoading}
                className="px-3 py-1.5 text-xs text-slate-400 hover:text-slate-200 transition-colors flex items-center gap-2 disabled:opacity-50"
              >
                <RefreshCw size={12} className={clsx(isLoading && "animate-spin")} />
                Refresh
              </button>
              <button
                onClick={() => setIsModalOpen(false)}
                className="px-4 py-1.5 text-xs font-medium text-slate-300 hover:bg-slate-800 rounded transition-colors"
              >
                Close
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
};
