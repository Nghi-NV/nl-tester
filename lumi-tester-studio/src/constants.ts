// Application Constants
export const APP_CONFIG = {
  DEFAULT_TIMEOUT: 10000, // 10 seconds
  DEFAULT_DELAY: 100, // Visual delay between steps
  DEBUG_MODE_DELAY: 1500, // Simulate API delay in debug mode
  CORS_PROXY_URL: 'https://corsproxy.io/?',
} as const;

export const AI_CONFIG = {
  DEFAULT_MODEL: 'gemini-2.0-flash',
  SYSTEM_INSTRUCTION: "You are an expert API Testing Assistant for 'Nexus API Tester'. You help users write YAML test flows, debug errors, and explain API concepts. The user might provide file contents using @mention. Always prefer YAML format for code output.",
  MENTION_REGEX: /@(\\S+)/g,
  INITIAL_MESSAGE: 'Hello! I am Nexus AI. I can help you write tests, explain flows, or debug issues.\\n\\nYou can mention files using **@filename** to verify or generate tests based on them.',
  CLEARED_MESSAGE: 'Chat history cleared.',
} as const;

export const FILE_CONFIG = {
  DEFAULT_FILE_CONTENT: 'name: New Test\\nsteps:\\n  - name: Example Step\\n    method: GET\\n    url: https://jsonplaceholder.typicode.com/posts/1',
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
