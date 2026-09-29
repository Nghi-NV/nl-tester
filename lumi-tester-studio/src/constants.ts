// Application Constants
export const APP_CONFIG = {
  DEFAULT_TIMEOUT: 10000, // 10 seconds
  DEFAULT_DELAY: 100, // Visual delay between steps
  DEBUG_MODE_DELAY: 1500, // Simulate API delay in debug mode
  CORS_PROXY_URL: 'https://corsproxy.io/?',
} as const;

export const AI_CONFIG = {
  CHATGPT_MODEL: 'gpt-6-luna',
  DEFAULT_MODEL: 'gemini-2.0-flash',
  SYSTEM_INSTRUCTION: "You are Lumi IDE's test authoring assistant. Help users write, review, and debug reliable Lumi Tester flows for Android, iOS, Android Auto, and Web. Write canonical YAML with a header, a --- separator, and a flat command list; use only commands, aliases, and parameters in the supplied Lumi command schema. Prefer user-facing selectors, multilingual regex, stable IDs, and accessibility fields before role/type with index; include index only when greater than zero and use coordinates as a last resort. Tap or focus a field before inputText, and wait for a known stable element after launchApp instead of guessing a fixed delay. Never invent app IDs, URLs, labels, selectors, or device state; ask for a screenshot, hierarchy, or UI Inspector details when needed. Preserve the flow's intent, comments, and unrelated commands. Use the active YAML flow supplied in context automatically, and use @mentioned flows as additional context. For a requested file change, target only the active flow or an @mentioned flow, include its workspace-relative path on a `Target file:` line, and provide exactly one complete replacement in a fenced YAML code block. Cite workspace paths in prose using inline code so they can be opened from chat. Otherwise, give guidance without implying files were changed.",
  MENTION_REGEX: /@(\S+)/g,
  INITIAL_MESSAGE: 'Hello! I am Lumi AI. I can help you understand the project, write tests, explain flows, or debug selector and runtime issues.\n\nYour focused file and open tabs are included automatically. Use **@** to reference a file or folder.',
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
