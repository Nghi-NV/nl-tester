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
  stepName: string;
  status: 'passed' | 'failed' | 'skipped' | 'cancelled';
  // Request Info
  url?: string;
  method?: string;
  requestHeaders?: Record<string, string>;
  requestBody?: any;
  // Response Info
  responseTime: number;
  responseStatus: number;
  responseHeaders?: Record<string, string>; // Headers dictionary
  responseBody?: any;
  error?: string;
  timestamp: number;
  depth?: number; // Visualization depth for nested flows
}

export interface TestRunResult {
  id: string;
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