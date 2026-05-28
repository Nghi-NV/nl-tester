// Application Constants
export const APP_CONFIG = {
  DEFAULT_TIMEOUT: 10000, // 10 seconds
  DEFAULT_DELAY: 100, // Visual delay between steps
  DEBUG_MODE_DELAY: 1500, // Simulate API delay in debug mode
  CORS_PROXY_URL: 'https://corsproxy.io/?',
} as const;

export const AI_CONFIG = {
  DEFAULT_MODEL: 'gemini-2.0-flash',
  SYSTEM_INSTRUCTION: "You are an expert Lumi Tester assistant. You help users write, validate, run, and debug Lumi YAML flows for Android, iOS, Android Auto, and Web automation. The user might provide file contents using @mention. Prefer the canonical header/---/steps format, stable selectors before coordinates, and concise YAML patches or complete YAML files.",
  MENTION_REGEX: /@(\S+)/g,
  INITIAL_MESSAGE: 'Hello! I am Lumi AI. I can help you write tests, explain flows, or debug selector and runtime issues.\n\nYou can mention files using **@filename** to verify or generate tests based on them.',
  CLEARED_MESSAGE: 'Chat history cleared.',
} as const;

export const FILE_CONFIG = {
  DEFAULT_FILE_CONTENT: 'platform: android\nappId: com.example.app\n---\n- launchApp\n- tap:\n    text: "Login"\n- inputText: "test@example.com"\n- see:\n    text: "Welcome"',
  YAML_EXTENSION: '.yaml',
} as const;

export const HTTP_METHODS = {
  GET: 'GET',
  POST: 'POST',
  PUT: 'PUT',
  DELETE: 'DELETE',
  PATCH: 'PATCH',
  HEAD: 'HEAD',
  OPTIONS: 'OPTIONS',
} as const;

export const STEP_STATUS = {
  PASSED: 'passed',
  FAILED: 'failed',
  SKIPPED: 'skipped',
  CANCELLED: 'cancelled',
} as const;

export const ERROR_MESSAGES = {
  FILE_NOT_FOUND: (filename: string) => `File '${filename}' not found.`,
  FLOW_PARSE_ERROR: (error: string) => `Flow parse error: ${error}`,
  YAML_PARSE_ERROR: 'Unknown YAML error',
  EMPTY_YAML: 'Empty YAML',
  TEST_CANCELLED: 'Cancelled by user',
  TIMEOUT: (timeout: number) => `Timeout after ${timeout}ms`,
  NETWORK_ERROR: 'Network Error (CORS)',
  STATUS_MISMATCH: (expected: number, actual: number) => `Expected status ${expected}, got ${actual}`,
  RESPONSE_TOO_SLOW: (duration: number, maxTime: number) => `Response too slow: ${duration}ms > ${maxTime}ms`,
  VERIFICATION_FAILED: (path: string, expected: any, actual: any) => `Verification failed for ${path}: expected ${expected}, got ${actual}`,
  AI_ERROR: (message: string) => `**Error**: Failed to generate response.\n\nDetails: ${message}\n\nPlease check your API Key and Model settings.`,
  AI_NO_RESPONSE: 'No response generated.',
} as const;
