export interface FileNode {
  id: string;
  name: string;
  type: 'file' | 'folder';
  children?: FileNode[];
  content?: string; // YAML content for files
  isOpen?: boolean; // For folder expansion
}

export interface EnvVar {
  key: string;
  value: string;
  enabled: boolean;
}

export interface TestConfig {
  baseUrl?: string;
  headers?: Record<string, string>;
  timeout?: number;
}

export interface TestStep {
  name: string;
  // Flow control
  flow?: string; // Reference to another file name
  // Standard Request
  method?: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH' | 'HEAD' | 'OPTIONS';
  url?: string;
  headers?: Record<string, string>;
  body?: any;
  extract?: Record<string, string>;
  verify?: {
    status?: number;
    responseTime?: number;
    [key: string]: any;
  };
}

export interface TestFlow {
  name: string;
  description?: string;
  config?: TestConfig;
  beforeTest?: TestStep[];
  steps?: TestStep[];
  flow?: string;
  afterTest?: TestStep[];
}

export interface StepResult {
  name: string;
  status: 'passed' | 'failed' | 'skipped' | 'running' | 'cancelled';
  duration?: number;
  logs?: string[];
  error?: string;
  timestamp: number;
  fileId?: string; // Path of the flow defining this step
  localIndex?: number; // Index within the specific flow/file
}

export interface TestResult {
  id: string;
  status: 'passed' | 'failed' | 'running';
  error?: string;
  fileId: string;
  fileName: string;
  timestamp: number;
  totalDuration: number;
  passed: number;
  failed: number;
  steps: StepResult[];
  batchId?: string; // ID for grouping folder runs
  folderName?: string; // Name of the folder executed
}

export type ViewMode = 'editor' | 'report' | 'settings';

export interface Snippet {
  label: string;
  code: string;
  description: string;
  type: 'keyword' | 'snippet';
}

// AI Types
export type AiRole = 'user' | 'model' | 'system';

export interface AiMessage {
  id: string;
  role: AiRole;
  content: string;
  timestamp: number;
}

export interface AiConfig {
  apiKey: string;
  model: string;
}
