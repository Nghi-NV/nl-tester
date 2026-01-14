import React, { useState, useEffect } from 'react';
import { createPortal } from 'react-dom';
import { StepResult } from '../types';
import { X, CheckCircle, XCircle, Clock, AlertCircle, Ban, ArrowUpRight, ArrowDownLeft, Globe, Copy, Check, Activity } from 'lucide-react';
import { clsx } from 'clsx';
import Editor, { useMonaco } from '@monaco-editor/react';
import { defineCodeverseTheme } from './editor/monacoUtils';
import ReactJson from 'react-json-view';

interface StepDetailModalProps {
  step: StepResult;
  onClose: () => void;
}

export const StepDetailModal: React.FC<StepDetailModalProps> = ({ step, onClose }) => {
  const [activeTab, setActiveTab] = useState<'request' | 'response' | 'error'>('response');
  const [reqSubTab, setReqSubTab] = useState<'body' | 'headers'>('body');
  const [resSubTab, setResSubTab] = useState<'body' | 'headers'>('body');
  const [copied, setCopied] = useState<string | null>(null);

  const monaco = useMonaco();

  // Ensure theme is defined
  useEffect(() => {
    if (monaco) {
      defineCodeverseTheme(monaco);
    }
  }, [monaco]);

  // Close on Escape key
  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };
    window.addEventListener('keydown', handleEscape);
    return () => window.removeEventListener('keydown', handleEscape);
  }, [onClose]);

  // Format request body (may be object or string)
  const formatRequestBody = (data: any): string => {
    if (data === undefined || data === null) {
      return '';
    }
    if (typeof data === 'string') {
      if (data.trim() === '') {
        return '';
      }
      try {
        const parsed = JSON.parse(data);
        return JSON.stringify(parsed, null, 2);
      } catch {
        return data;
      }
    }
    if (typeof data === 'object') {
      try {
        return JSON.stringify(data, null, 2);
      } catch (e) {
        return String(data);
      }
    }
    return String(data);
  };

  // Parse response body for react-json-view (expects object/array)
  const parseResponseBody = (data: any): any => {
    if (!data || data === '') {
      return null;
    }
    if (typeof data === 'string') {
      try {
        return JSON.parse(data);
      } catch {
        return data;
      }
    }
    if (typeof data === 'object') {
      return data;
    }
    return null;
  };

  const copyToClipboard = async (text: string, type: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(type);
      setTimeout(() => setCopied(null), 2000);
    } catch (err) {
      console.error('Failed to copy:', err);
    }
  };

  const requestBody = React.useMemo(() => {
    const formatted = formatRequestBody(step.requestBody);
    return formatted || 'No request body';
  }, [step.requestBody]);

  const responseBodyParsed = React.useMemo(() => {
    return parseResponseBody(step.responseBody);
  }, [step.responseBody]);

  const responseBodyText = React.useMemo(() => {
    if (!step.responseBody || step.responseBody === '') {
      return 'No response body';
    }
    return typeof step.responseBody === 'string' ? step.responseBody : JSON.stringify(step.responseBody, null, 2);
  }, [step.responseBody]);

  // Use portal to render outside of any stacking context
  const modalContent = (
    <>
      {/* Backdrop */}
      <div
        className="fixed inset-0 z-[9999] bg-black/60 backdrop-blur-sm animate-in fade-in duration-200"
        onClick={onClose}
      />

      {/* Modal */}
      <div className="fixed inset-0 z-[10000] flex items-center justify-center p-4 pointer-events-none animate-in fade-in duration-200">
        <div
          className="w-full max-w-6xl h-full max-h-[95vh] bg-slate-900 border border-slate-700 rounded-xl shadow-2xl flex flex-col overflow-hidden pointer-events-auto animate-in zoom-in-95 duration-200
                     md:max-w-4xl lg:max-w-6xl"
          onClick={(e) => e.stopPropagation()}
        >
          {/* Header */}
          <div className="p-4 md:p-5 border-b border-slate-800 flex items-center justify-between bg-slate-950/50 flex-shrink-0">
            <div className="flex items-center gap-3 md:gap-4 min-w-0 flex-1">
              <div className={clsx(
                "p-2 rounded-lg flex-shrink-0",
                step.status === 'passed' ? "bg-emerald-500/10 text-emerald-400" :
                  step.status === 'failed' ? "bg-rose-500/10 text-rose-400" :
                    step.status === 'skipped' ? "bg-yellow-500/10 text-yellow-400" : "bg-slate-800 text-slate-400"
              )}>
                {step.status === 'passed' ? <CheckCircle size={20} className="md:w-6 md:h-6" /> :
                  step.status === 'failed' ? <XCircle size={20} className="md:w-6 md:h-6" /> :
                    step.status === 'skipped' ? <AlertCircle size={20} className="md:w-6 md:h-6" /> : <Ban size={20} className="md:w-6 md:h-6" />}
              </div>
              <div className="min-w-0 flex-1">
                <h3 className="text-base md:text-lg font-bold text-slate-100 truncate">{step.stepName}</h3>
                <div className="flex items-center gap-2 md:gap-4 text-xs text-slate-500 mt-1 flex-wrap">
                  <span className="flex items-center gap-1.5 bg-slate-800 px-2 py-0.5 rounded text-cyan-400 font-mono text-xs">
                    {step.method || 'GET'}
                  </span>
                  <span className="flex items-center gap-1 font-mono text-slate-400 truncate max-w-full" title={step.url}>
                    <Globe size={12} className="flex-shrink-0" />
                    <span className="truncate">{step.url || 'No URL captured'}</span>
                  </span>
                </div>
              </div>
            </div>
            <button
              onClick={onClose}
              className="p-2 hover:bg-slate-800 rounded-lg text-slate-400 hover:text-white transition-colors flex-shrink-0 ml-2"
              aria-label="Close"
            >
              <X size={20} className="md:w-6 md:h-6" />
            </button>
          </div>

          {/* Main Tabs */}
          <div className="flex border-b border-slate-800 bg-slate-950/30 flex-shrink-0 overflow-x-auto">
            <button
              onClick={() => setActiveTab('request')}
              className={clsx(
                "px-4 md:px-6 py-2 md:py-3 text-xs md:text-sm font-bold border-b-2 transition-colors flex items-center gap-1.5 md:gap-2 whitespace-nowrap",
                activeTab === 'request' ? "border-cyan-500 text-cyan-400" : "border-transparent text-slate-500 hover:text-slate-300"
              )}
            >
              <ArrowUpRight size={14} className="md:w-4 md:h-4" />
              <span>Request</span>
            </button>
            <button
              onClick={() => setActiveTab('response')}
              className={clsx(
                "px-4 md:px-6 py-2 md:py-3 text-xs md:text-sm font-bold border-b-2 transition-colors flex items-center gap-1.5 md:gap-2 whitespace-nowrap",
                activeTab === 'response' ? "border-emerald-500 text-emerald-400" : "border-transparent text-slate-500 hover:text-slate-300"
              )}
            >
              <ArrowDownLeft size={14} className="md:w-4 md:h-4" />
              <span>Response</span>
            </button>
            {step.error && (
              <button
                onClick={() => setActiveTab('error')}
                className={clsx(
                  "px-4 md:px-6 py-2 md:py-3 text-xs md:text-sm font-bold border-b-2 transition-colors flex items-center gap-1.5 md:gap-2 whitespace-nowrap",
                  activeTab === 'error' ? "border-rose-500 text-rose-400" : "border-transparent text-slate-500 hover:text-rose-400"
                )}
              >
                <AlertCircle size={14} className="md:w-4 md:h-4" />
                <span>Error</span>
              </button>
            )}
          </div>

          {/* Content Area */}
          <div className="flex-1 overflow-hidden bg-slate-950 relative flex flex-col min-h-0">
            {/* Request View */}
            {activeTab === 'request' && (
              <div className="flex flex-col h-full">
                <div className="flex items-center gap-2 p-2 bg-slate-900 border-b border-slate-800 text-xs text-slate-400 flex-shrink-0">
                  <button
                    onClick={() => setReqSubTab('body')}
                    className={clsx(
                      "px-3 py-1 rounded transition-colors text-xs",
                      reqSubTab === 'body' ? "bg-slate-700 text-white" : "hover:bg-slate-800"
                    )}
                  >
                    Body
                  </button>
                  <button
                    onClick={() => setReqSubTab('headers')}
                    className={clsx(
                      "px-3 py-1 rounded transition-colors text-xs",
                      reqSubTab === 'headers' ? "bg-slate-700 text-white" : "hover:bg-slate-800"
                    )}
                  >
                    Headers
                  </button>
                  {reqSubTab === 'body' && (
                    <button
                      onClick={() => copyToClipboard(requestBody, 'request-body')}
                      className="ml-auto px-2 py-1 rounded hover:bg-slate-800 text-slate-400 hover:text-cyan-400 transition-colors flex items-center gap-1"
                      title="Copy to clipboard"
                    >
                      {copied === 'request-body' ? <Check size={14} /> : <Copy size={14} />}
                    </button>
                  )}
                </div>
                <div className="flex-1 relative min-h-0">
                  {reqSubTab === 'body' ? (
                    <Editor
                      key={`request-body-${step.stepName}-${step.timestamp}`}
                      height="100%"
                      defaultLanguage="json"
                      language="json"
                      theme="codeverse-dark"
                      value={requestBody}
                      onChange={() => { }}
                      loading={<div className="text-slate-400 p-4">Loading editor...</div>}
                      options={{
                        readOnly: true,
                        minimap: { enabled: false },
                        fontSize: 12,
                        padding: { top: 16, bottom: 16 },
                        automaticLayout: true,
                        wordWrap: 'on',
                        scrollBeyondLastLine: false
                      }}
                    />
                  ) : (
                    <div className="p-3 md:p-4 overflow-y-auto h-full">
                      <div className="bg-slate-900 rounded border border-slate-800 overflow-hidden">
                        <table className="w-full text-left text-xs">
                          <thead className="bg-slate-950 text-slate-400 font-bold uppercase">
                            <tr>
                              <th className="px-3 md:px-4 py-2 border-r border-slate-800 w-1/3">Header Name</th>
                              <th className="px-3 md:px-4 py-2">Value</th>
                            </tr>
                          </thead>
                          <tbody className="divide-y divide-slate-800 text-slate-300 font-mono">
                            {step.requestHeaders ? Object.entries(step.requestHeaders).map(([k, v]) => (
                              <tr key={k}>
                                <td className="px-3 md:px-4 py-2 border-r border-slate-800 text-cyan-400 break-words">{k}</td>
                                <td className="px-3 md:px-4 py-2 break-all">{v}</td>
                              </tr>
                            )) : (
                              <tr>
                                <td colSpan={2} className="p-4 text-center text-slate-500">No headers captured</td>
                              </tr>
                            )}
                          </tbody>
                        </table>
                      </div>
                    </div>
                  )}
                </div>
              </div>
            )}

            {/* Response View */}
            {activeTab === 'response' && (
              <div className="flex flex-col h-full">
                <div className="p-2 md:p-3 bg-slate-900 border-b border-slate-800 flex items-center justify-between text-xs flex-shrink-0 flex-wrap gap-2">
                  <div className="flex gap-3 md:gap-4 flex-wrap">
                    <div className="flex items-center gap-2">
                      <Activity size={12} className="md:w-3.5 md:h-3.5 text-slate-500" />
                      <span className={clsx(
                        "font-bold font-mono text-xs md:text-sm",
                        step.responseStatus >= 400 ? "text-rose-400" : "text-emerald-400"
                      )}>
                        {step.responseStatus} {step.responseStatus >= 200 && step.responseStatus < 300 ? 'OK' : ''}
                      </span>
                    </div>
                    <div className="flex items-center gap-2">
                      <Clock size={12} className="md:w-3.5 md:h-3.5 text-slate-500" />
                      <span className="font-mono text-cyan-400 text-xs md:text-sm">{step.responseTime}ms</span>
                    </div>
                  </div>
                  <div className="flex items-center gap-2">
                    <button
                      onClick={() => setResSubTab('body')}
                      className={clsx(
                        "px-3 py-1 rounded transition-colors text-xs",
                        resSubTab === 'body' ? "bg-slate-700 text-white" : "hover:bg-slate-800"
                      )}
                    >
                      Body
                    </button>
                    <button
                      onClick={() => setResSubTab('headers')}
                      className={clsx(
                        "px-3 py-1 rounded transition-colors text-xs",
                        resSubTab === 'headers' ? "bg-slate-700 text-white" : "hover:bg-slate-800"
                      )}
                    >
                      Headers
                    </button>
                    {resSubTab === 'body' && (
                      <button
                        onClick={() => copyToClipboard(responseBodyText, 'response-body')}
                        className="px-2 py-1 rounded hover:bg-slate-800 text-slate-400 hover:text-cyan-400 transition-colors flex items-center gap-1"
                        title="Copy to clipboard"
                      >
                        {copied === 'response-body' ? <Check size={14} /> : <Copy size={14} />}
                      </button>
                    )}
                  </div>
                </div>
                <div className="flex-1 relative min-h-0 overflow-auto">
                  {resSubTab === 'body' ? (
                    responseBodyParsed !== null && typeof responseBodyParsed === 'object' ? (
                      <div className="p-4 h-full overflow-auto">
                        <ReactJson
                          src={responseBodyParsed}
                          theme="monokai"
                          collapsed={2}
                          displayDataTypes={false}
                          displayObjectSize={true}
                          enableClipboard={false}
                          style={{
                            backgroundColor: 'transparent',
                            fontSize: '13px',
                            fontFamily: 'monospace'
                          }}
                        />
                      </div>
                    ) : (
                      <Editor
                        key={`response-body-${step.stepName}-${step.timestamp}`}
                        height="100%"
                        defaultLanguage="text"
                        language="text"
                        theme="codeverse-dark"
                        value={responseBodyText}
                        onChange={() => { }}
                        loading={<div className="text-slate-400 p-4">Loading editor...</div>}
                        options={{
                          readOnly: true,
                          minimap: { enabled: false },
                          fontSize: 12,
                          padding: { top: 16, bottom: 16 },
                          wordWrap: 'on',
                          automaticLayout: true,
                          scrollBeyondLastLine: false
                        }}
                      />
                    )
                  ) : (
                    <div className="p-3 md:p-4 overflow-y-auto h-full">
                      <div className="bg-slate-900 rounded border border-slate-800 overflow-hidden">
                        <table className="w-full text-left text-xs">
                          <thead className="bg-slate-950 text-slate-400 font-bold uppercase">
                            <tr>
                              <th className="px-3 md:px-4 py-2 border-r border-slate-800 w-1/3">Header Name</th>
                              <th className="px-3 md:px-4 py-2">Value</th>
                            </tr>
                          </thead>
                          <tbody className="divide-y divide-slate-800 text-slate-300 font-mono">
                            {step.responseHeaders ? Object.entries(step.responseHeaders).map(([k, v]) => (
                              <tr key={k}>
                                <td className="px-3 md:px-4 py-2 border-r border-slate-800 text-emerald-400 break-words">{k}</td>
                                <td className="px-3 md:px-4 py-2 break-all">{v}</td>
                              </tr>
                            )) : (
                              <tr>
                                <td colSpan={2} className="p-4 text-center text-slate-500">No headers captured</td>
                              </tr>
                            )}
                          </tbody>
                        </table>
                      </div>
                    </div>
                  )}
                </div>
              </div>
            )}

            {/* Error View */}
            {activeTab === 'error' && step.error && (
              <div className="p-4 md:p-6 overflow-y-auto h-full">
                <div className="bg-rose-950/20 border border-rose-900/50 rounded-lg p-4">
                  <div className="flex items-start gap-3">
                    <XCircle className="text-rose-500 shrink-0 mt-0.5" size={18} />
                    <div className="flex-1 min-w-0">
                      <h4 className="text-rose-400 font-bold mb-2">Error Details</h4>
                      <p className="text-rose-300/80 text-sm leading-relaxed whitespace-pre-wrap font-mono break-words">
                        {step.error}
                      </p>
                    </div>
                  </div>
                </div>
              </div>
            )}
          </div>

          {/* Footer */}
          <div className="p-3 md:p-4 bg-slate-900 border-t border-slate-800 flex justify-end flex-shrink-0">
            <button
              onClick={onClose}
              className="px-4 md:px-6 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 text-sm font-medium rounded-lg transition-colors"
            >
              Close
            </button>
          </div>
        </div>
      </div>
    </>
  );

  // Render using portal to document.body to avoid stacking context issues
  return createPortal(modalContent, document.body);
};
