import React from 'react';
import { X, Book, Code, Box } from 'lucide-react';

interface Props {
  onClose: () => void;
}

export const HelpModal: React.FC<Props> = ({ onClose }) => {
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-slate-950/80 backdrop-blur-sm p-4">
      <div className="bg-slate-900 border border-borderGlass rounded-xl shadow-2xl w-full max-w-4xl flex flex-col max-h-[85vh] animate-in fade-in zoom-in-95 duration-200">
        
        {/* Header */}
        <div className="flex items-center justify-between p-6 border-b border-borderGlass">
          <div className="flex items-center gap-3">
             <div className="p-2 bg-cyan-500/10 rounded-lg">
                <Book className="text-cyan-400" size={24} />
             </div>
             <div>
                <h2 className="text-xl font-bold text-white">Documentation</h2>
                <p className="text-slate-400 text-sm">Command reference and syntax guide</p>
             </div>
          </div>
          <button onClick={onClose} className="text-slate-400 hover:text-white transition-colors hover:bg-slate-800 p-2 rounded-full">
            <X size={24} />
          </button>
        </div>
        
        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6 space-y-8">
            
            {/* Section 1: Structure */}
            <section>
                <h3 className="text-lg font-semibold text-cyan-400 mb-4 flex items-center gap-2">
                    <Box size={18} /> Basic Structure
                </h3>
                <div className="bg-slate-950 p-4 rounded-lg border border-slate-800">
                    <pre className="font-mono text-xs text-slate-300 leading-5">
{`name: Name of your test flow
description: Optional description
steps:
  - name: Step Name
    method: GET | POST | PUT | DELETE
    url: https://api.example.com/endpoint
    headers:
      Content-Type: application/json
    body:
      key: value`}
                    </pre>
                </div>
            </section>

            {/* Section 2: Variables */}
            <section>
                <h3 className="text-lg font-semibold text-cyan-400 mb-4 flex items-center gap-2">
                    <Code size={18} /> Variables & Extraction
                </h3>
                <p className="text-sm text-slate-400 mb-4">
                    Use <code className="bg-slate-800 px-1 rounded text-cyan-300">{'{{variable_name}}'}</code> to inject values.
                    Extract values from responses to use in subsequent steps.
                </p>
                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div className="bg-slate-950 p-4 rounded-lg border border-slate-800">
                        <p className="text-xs font-bold text-slate-500 uppercase mb-2">Extraction</p>
                        <pre className="font-mono text-xs text-slate-300 leading-5">
{`extract:
  user_id: body.data.id
  token: body.auth_token
  header_val: headers.x-trace-id`}
                        </pre>
                    </div>
                    <div className="bg-slate-950 p-4 rounded-lg border border-slate-800">
                        <p className="text-xs font-bold text-slate-500 uppercase mb-2">Usage</p>
                        <pre className="font-mono text-xs text-slate-300 leading-5">
{`url: https://api.com/users/{{user_id}}
headers:
  Authorization: Bearer {{token}}`}
                        </pre>
                    </div>
                </div>
            </section>

            {/* Section 3: Verification */}
            <section>
                <h3 className="text-lg font-semibold text-cyan-400 mb-4 flex items-center gap-2">
                    <Code size={18} /> Assertions & Verify
                </h3>
                <div className="bg-slate-950 p-4 rounded-lg border border-slate-800">
                    <pre className="font-mono text-xs text-slate-300 leading-5">
{`verify:
  status: 200                  # Check HTTP Status
  responseTime: 500            # Max duration in ms
  body.success: true           # Check boolean path
  body.data.role: admin        # Check string path
  body.items[0].id: 12         # Array access`}
                    </pre>
                </div>
            </section>

             {/* Section 4: Tips */}
            <section className="bg-cyan-900/10 border border-cyan-500/20 p-4 rounded-lg">
                <h4 className="text-sm font-bold text-cyan-300 mb-2">Pro Tips</h4>
                <ul className="text-sm text-slate-400 space-y-2 list-disc pl-4">
                    <li>Click the <b>Play</b> icon on a Folder to run all tests inside it sequentially.</li>
                    <li>Use the <b>Snippets</b> dropdown in the editor to quickly insert common patterns.</li>
                    <li>Environment variables persist across test runs until the page is refreshed.</li>
                </ul>
            </section>
        </div>
      </div>
    </div>
  );
};