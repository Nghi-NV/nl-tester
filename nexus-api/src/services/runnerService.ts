import jsyaml from 'js-yaml';
import { fetch } from '@tauri-apps/plugin-http';
import pako from 'pako';
import { EnvVar, TestFlow, StepResult, TestRunResult, TestStep, TestConfig } from '../types';
import { useEditorStore, useFileStore } from '../stores';
import { resolveMock, isMockKey } from './mockService';
import { APP_CONFIG, ERROR_MESSAGES, STEP_STATUS, HTTP_METHODS } from '../constants';
import { generateRunId } from '../utils/idGenerator';

// Basic interpolation {{variable}} or {{$mock.key}}
const interpolate = (text: string, env: Record<string, any>): string => {
  return text.replace(/\{\{([^}]+)\}\}/g, (_, key) => {
    const trimmedKey = key.trim();
    if (isMockKey(trimmedKey)) {
      return String(resolveMock(trimmedKey));
    }
    return env[trimmedKey] !== undefined ? String(env[trimmedKey]) : `{{${trimmedKey}}}`;
  });
};

// Deep interpolate
const deepInterpolate = (obj: any, env: Record<string, any>): any => {
  if (typeof obj === 'string') return interpolate(obj, env);
  if (Array.isArray(obj)) return obj.map(item => deepInterpolate(item, env));
  if (typeof obj === 'object' && obj !== null) {
    const result: any = {};
    for (const key in obj) {
      result[key] = deepInterpolate(obj[key], env);
    }
    return result;
  }
  return obj;
};

const getValueByPath = (obj: any, path: string) => {
  return path.split('.').reduce((acc, part) => acc && acc[part], obj);
};

// Execute a single step
const executeStep = async (
  rawStep: TestStep,
  env: Record<string, any>,
  config: TestConfig,
  depth: number,
  signal?: AbortSignal
): Promise<StepResult> => {
  const stepStart = Date.now();

  // Update Store to show execution line in editor
  if (rawStep.name) {
    useEditorStore.getState().setActiveStepName(rawStep.name);
  }

  // Interpolate Step Data with Environment and Mock
  const step = deepInterpolate(rawStep, env) as TestStep;

  let stepResult: StepResult = {
    stepName: step.name || 'Unnamed Step',
    status: STEP_STATUS.PASSED,
    responseTime: 0,
    responseStatus: 0,
    timestamp: Date.now(),
    depth
  };

  try {
    if (signal?.aborted) {
      throw new Error('Test run cancelled');
    }

    // Merge URL with BaseURL if present
    let url = step.url || '';
    if (config.baseUrl && !url.startsWith('http')) {
      url = `${config.baseUrl}${url.startsWith('/') ? '' : '/'}${url}`;
    }

    // Prepare Headers
    let headers = { ...config.headers, ...step.headers };

    // Prepare Body and Method
    const method = step.method || HTTP_METHODS.GET;
    let body = step.body ? JSON.stringify(step.body) : undefined;

    // CLEANUP: Remove Content-Type and Body for GET/HEAD requests to avoid CORS/Protocol errors
    if (method === HTTP_METHODS.GET || method === HTTP_METHODS.HEAD) {
      body = undefined;
      // Create a new headers object without Content-Type if it exists
      const newHeaders: Record<string, string> = {};
      Object.keys(headers).forEach(key => {
        if (key.toLowerCase() !== 'content-type') {
          newHeaders[key] = headers[key];
        }
      });
      headers = newHeaders;
    }

    // Add standard headers based on Postman and browser standards
    // Only add if not already set by user (allow override)
    // Optimized for API testing while avoiding WAF/bot detection
    const defaultBrowserHeaders: Record<string, string> = {
      // Standard Accept header for API calls (Postman uses */*, but we use more specific)
      'Accept': 'application/json, text/plain, */*',
      // Language preferences (standard browser header)
      'Accept-Language': 'en-US,en;q=0.9',
      // Note: Accept-Encoding removed - tauri-plugin-http may not auto-decompress
      // Server will send uncompressed response
      // Connection management (Postman and browsers use keep-alive)
      'Connection': 'keep-alive',
      // Cache control (Postman sends no-cache for fresh requests)
      'Cache-Control': 'no-cache',
      // Modern browser security headers (Sec-Fetch-*)
      'Sec-Fetch-Mode': 'cors',
      'Sec-Fetch-Site': 'cross-site',
      'Sec-Fetch-Dest': 'empty', // API calls use 'empty', not 'document'
      'Sec-Fetch-User': '?1', // Indicates user-initiated request
      // Client Hints headers (modern browsers send these, helps with Cloudflare)
      'Sec-CH-UA': '"Chromium";v="122", "Not(A:Brand";v="8", "Google Chrome";v="122"',
      'Sec-CH-UA-Mobile': '?0',
      'Sec-CH-UA-Platform': '"macOS"',
      'Sec-CH-UA-Platform-Version': '"14.1.0"',
      'Sec-CH-UA-Arch': '"x86"',
      'Sec-CH-UA-Bitness': '"64"',
      'Sec-CH-UA-Full-Version': '"122.0.0.0"',
      'Sec-CH-UA-Full-Version-List': '"Chromium";v="122.0.6261.128", "Not(A:Brand";v="8.0.0.0", "Google Chrome";v="122.0.6261.128"',
      'Sec-CH-UA-Model': '""'
    };

    // Merge default headers only if not already present (case-insensitive check)
    const headerKeysLower = new Set(Object.keys(headers).map(k => k.toLowerCase()));
    for (const [key, value] of Object.entries(defaultBrowserHeaders)) {
      if (!headerKeysLower.has(key.toLowerCase())) {
        headers[key] = value;
      }
    }

    // Ensure User-Agent is set to a real browser one to avoid WAF blocking
    // Cloudflare hates "PostmanRuntime" or empty UAs sometimes
    if (!headers['User-Agent'] && !headers['user-agent']) {
      headers['User-Agent'] = 'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Safari/537.36';
    }

    // Capture Request Info
    stepResult.url = url;
    stepResult.method = method;
    stepResult.requestHeaders = headers;
    // Store request body - try to parse if it's JSON string, otherwise keep as string
    if (body) {
      try {
        // Try to parse as JSON to store as object for better display
        stepResult.requestBody = JSON.parse(body);
      } catch {
        // Not JSON, store as string
        stepResult.requestBody = body;
      }
    } else {
      stepResult.requestBody = undefined;
    }

    // Timeout Logic
    const timeout = config.timeout || APP_CONFIG.DEFAULT_TIMEOUT;

    const abortController = new AbortController();
    const timeoutId = setTimeout(() => abortController.abort(), timeout);

    // Combine abort signals
    if (signal) {
      signal.addEventListener('abort', () => abortController.abort());
    }

    try {
      console.log(`[DEBUG] Making HTTP request: ${url}`);
      console.log(`[DEBUG] Headers:`, JSON.stringify(headers, null, 2));
      console.log(`[DEBUG] Request Body:`, body);

      // Use tauri-plugin-http fetch API
      const response = await fetch(url, {
        method,
        headers,
        body,
        signal: abortController.signal
      });

      clearTimeout(timeoutId);

      const duration = Date.now() - stepStart;
      stepResult.responseTime = duration;
      stepResult.responseStatus = response.status;

      // Capture Response Headers
      const responseHeaders: Record<string, string> = {};
      response.headers.forEach((value, key) => {
        responseHeaders[key] = value;
      });
      stepResult.responseHeaders = responseHeaders;

      // Parse Body based on Content-Type
      const contentType = (responseHeaders['content-type'] || '').toLowerCase();
      const contentEncoding = (responseHeaders['content-encoding'] || '').toLowerCase();
      let responseBody: any = null;

      // Read body and handle decompression if needed
      try {
        // Read as arrayBuffer first to handle compression
        const arrayBuffer = await response.arrayBuffer();
        let bodyText: string = '';

        // Check if response is compressed
        if (contentEncoding && (contentEncoding.includes('gzip') || contentEncoding.includes('deflate'))) {
          try {
            // Decompress using pako
            const decompressed = pako.inflate(new Uint8Array(arrayBuffer), { to: 'string' });
            bodyText = decompressed;
            console.log(`[DEBUG] Successfully decompressed ${contentEncoding} response`);
          } catch (decompressError: any) {
            console.error(`[DEBUG] Failed to decompress ${contentEncoding}:`, decompressError);
            // Try to read as text anyway (might be raw text despite encoding header)
            try {
              bodyText = new TextDecoder().decode(arrayBuffer);
            } catch {
              bodyText = '';
            }
          }
        } else {
          // Not compressed, decode directly
          bodyText = new TextDecoder().decode(arrayBuffer);
        }

        // Store response body as string (for JSON, keep as formatted string)
        // Parse JSON only for verification/extraction, but store the original string
        if (contentType.includes("application/json") && bodyText) {
          try {
            // Validate JSON by parsing, but store the original string
            const parsed = JSON.parse(bodyText);
            // Store as formatted JSON string for display
            responseBody = JSON.stringify(parsed, null, 2);
            console.log(`[DEBUG] Successfully parsed and formatted JSON response`);
          } catch (e) {
            console.error('[DEBUG] JSON parse error:', e);
            // If not valid JSON, store as raw text
            responseBody = bodyText;
          }
        } else {
          responseBody = bodyText || '';
        }
      } catch (e: any) {
        console.error('[DEBUG] Error reading response body:', e);
        responseBody = `Error reading response: ${e.message || e}`;
      }

      // Store response body as string
      stepResult.responseBody = responseBody;

      // Parse for verification/extraction (if needed)
      let parsedResponseBody: any = null;
      if (contentType.includes("application/json") && typeof responseBody === 'string') {
        try {
          parsedResponseBody = JSON.parse(responseBody);
        } catch {
          parsedResponseBody = null;
        }
      }

      // Verify
      if (step.verify) {
        const actualStatus = stepResult.responseStatus;
        if (step.verify.status && actualStatus !== step.verify.status) {
          throw new Error(ERROR_MESSAGES.STATUS_MISMATCH(step.verify.status, actualStatus));
        }
        if (step.verify.responseTime && duration > step.verify.responseTime) {
          throw new Error(ERROR_MESSAGES.RESPONSE_TOO_SLOW(duration, step.verify.responseTime));
        }

        for (const key in step.verify) {
          if (key.startsWith('body.')) {
            const path = key.substring(5);
            // Use parsed response body for verification
            const bodyForVerification = parsedResponseBody || responseBody;
            const actual = getValueByPath(bodyForVerification, path);
            let expected = step.verify[key];

            // Interpolate expected value if it contains variables (e.g., {{user_id}})
            if (typeof expected === 'string' && expected.includes('{{')) {
              expected = interpolate(expected, env);
              // Try to convert to number if actual is number
              if (typeof actual === 'number') {
                const numExpected = Number(expected);
                if (!isNaN(numExpected)) {
                  expected = numExpected;
                }
              }
              // If expected is still a string like "[object Object]", try to get value from env directly
              if (typeof expected === 'string' && expected === '[object Object]') {
                // Extract variable name from original string (e.g., "{{author_id}}" -> "author_id")
                const varMatch = step.verify[key].match(/\{\{([^}]+)\}\}/);
                if (varMatch && varMatch[1]) {
                  const varName = varMatch[1].trim();
                  const envValue = env[varName];
                  if (envValue !== undefined) {
                    expected = envValue;
                  }
                }
              }
            }

            // Debug logging
            if (actual === undefined) {
              console.log(`[DEBUG] Verification failed - path "${path}" not found in response body:`, responseBody);
            }

            // Compare with type coercion for numbers
            if (actual != expected) {
              throw new Error(ERROR_MESSAGES.VERIFICATION_FAILED(path, expected, actual));
            }
          }
        }
      }

      // Extract
      if (step.extract) {
        for (const key in step.extract) {
          const path = step.extract[key];
          if (path.startsWith('body.')) {
            // Use parsed response body for extraction
            const bodyForExtraction = parsedResponseBody || responseBody;
            const val = getValueByPath(bodyForExtraction, path.substring(5));
            if (val !== undefined) env[key] = val;
          }
        }
      }

    } catch (err: any) {
      console.error("HTTP Request Error:", err);
      clearTimeout(timeoutId);
      throw err;
    }

  } catch (err: any) {
    if (err.name === 'AbortError' || signal?.aborted) {
      if (signal?.aborted) {
        stepResult.status = STEP_STATUS.CANCELLED;
        stepResult.error = ERROR_MESSAGES.TEST_CANCELLED;
      } else {
        stepResult.status = STEP_STATUS.FAILED;
        stepResult.error = ERROR_MESSAGES.TIMEOUT(config.timeout || APP_CONFIG.DEFAULT_TIMEOUT);
      }
    } else {
      stepResult.status = STEP_STATUS.FAILED;
      stepResult.error = err.message || ERROR_MESSAGES.NETWORK_ERROR;
    }
  }

  return stepResult;
};

// Recursive function to process a list of steps, handling 'flow' calls
const processSteps = async (
  steps: TestStep[],
  env: Record<string, any>,
  config: TestConfig,
  depth: number,
  onStepComplete: (stepRes: StepResult) => void,
  signal?: AbortSignal
): Promise<{ passed: number; failed: number }> => {
  let passed = 0;
  let failed = 0;

  for (const step of steps) {
    if (signal?.aborted) break;

    if (step.flow) {
      // Handle Nested Flow
      const flowContent = useFileStore.getState().getFileContentByName(step.flow);

      const flowStartResult: StepResult = {
        stepName: `Flow: ${step.flow} (Start)`,
        status: STEP_STATUS.PASSED,
        responseTime: 0,
        responseStatus: 0,
        timestamp: Date.now(),
        depth
      };
      onStepComplete(flowStartResult); // Signal flow start

      if (!flowContent) {
        onStepComplete({
          ...flowStartResult,
          stepName: `Flow: ${step.flow} (Error)`,
          status: STEP_STATUS.FAILED,
          error: ERROR_MESSAGES.FILE_NOT_FOUND(step.flow)
        });
        failed++;
        continue;
      }

      try {
        const nestedFlow = jsyaml.load(flowContent) as TestFlow;
        const nestedConfig = { ...config, ...nestedFlow.config }; // Merge config

        // Before Test Hook
        if (nestedFlow.beforeTest) {
          const res = await processSteps(nestedFlow.beforeTest, env, nestedConfig, depth + 1, onStepComplete, signal);
          passed += res.passed; failed += res.failed;
        }

        // Main Steps
        if (nestedFlow.steps) {
          const res = await processSteps(nestedFlow.steps, env, nestedConfig, depth + 1, onStepComplete, signal);
          passed += res.passed; failed += res.failed;
        } else if (nestedFlow.flow) {
          const syntheticStep: TestStep = {
            name: `Flow Reference: ${nestedFlow.flow}`,
            flow: nestedFlow.flow
          };
          const res = await processSteps([syntheticStep], env, nestedConfig, depth + 1, onStepComplete, signal);
          passed += res.passed; failed += res.failed;
        }

        // After Test Hook
        if (nestedFlow.afterTest) {
          const res = await processSteps(nestedFlow.afterTest, env, nestedConfig, depth + 1, onStepComplete, signal);
          passed += res.passed; failed += res.failed;
        }

      } catch (e: any) {
        onStepComplete({
          ...flowStartResult,
          stepName: `Flow: ${step.flow} (Parse Error)`,
          status: STEP_STATUS.FAILED,
          error: ERROR_MESSAGES.FLOW_PARSE_ERROR(e.message)
        });
        failed++;
      }

    } else {
      // Handle Standard Step
      const result = await executeStep(step, env, config, depth, signal);
      if (result.status === STEP_STATUS.PASSED) passed++;
      else if (result.status === STEP_STATUS.FAILED) failed++;

      onStepComplete(result);
      if (!signal?.aborted) await new Promise(r => setTimeout(r, APP_CONFIG.DEFAULT_DELAY));
    }
  }

  return { passed, failed };
};

export const runTestFlow = async (
  yamlContent: string,
  envVars: EnvVar[],
  fileId: string,
  fileName: string,
  onStepComplete: (stepRes: StepResult) => void,
  signal?: AbortSignal
): Promise<TestRunResult> => {
  const runId = generateRunId();
  const startTime = Date.now();
  const results: StepResult[] = [];

  const trackingCallback = (res: StepResult) => {
    results.push(res);
    onStepComplete(res);
  };

  // Initial Env
  let runtimeEnv: Record<string, any> = {};
  envVars.forEach(v => {
    if (v.enabled) runtimeEnv[v.key] = v.value;
  });

  try {
    const flow = jsyaml.load(yamlContent) as TestFlow;

    if (!flow || (!flow.steps && !flow.flow)) {
      if (!flow) throw new Error(ERROR_MESSAGES.EMPTY_YAML);
    }

    const config = flow.config || {};
    let passed = 0;
    let failed = 0;

    if (flow.beforeTest) {
      const res = await processSteps(flow.beforeTest, runtimeEnv, config, 0, trackingCallback, signal);
      passed += res.passed; failed += res.failed;
    }

    if (flow.steps && !signal?.aborted) {
      const res = await processSteps(flow.steps, runtimeEnv, config, 0, trackingCallback, signal);
      passed += res.passed; failed += res.failed;
    } else if (flow.flow && !signal?.aborted) {
      const syntheticStep: TestStep = {
        name: `Flow Reference: ${flow.flow}`,
        flow: flow.flow
      };
      const res = await processSteps([syntheticStep], runtimeEnv, config, 0, trackingCallback, signal);
      passed += res.passed; failed += res.failed;
    }

    if (flow.afterTest && !signal?.aborted) {
      const res = await processSteps(flow.afterTest, runtimeEnv, config, 0, trackingCallback, signal);
      passed += res.passed; failed += res.failed;
    }

    return {
      id: runId,
      fileId,
      fileName,
      timestamp: Date.now(),
      totalDuration: Date.now() - startTime,
      passed,
      failed,
      steps: results
    };

  } catch (parseError: any) {
    return {
      id: runId,
      fileId,
      fileName,
      timestamp: Date.now(),
      totalDuration: 0,
      passed: 0,
      failed: 1,
      steps: [{
        stepName: 'YAML Parse',
        status: STEP_STATUS.FAILED,
        responseTime: 0,
        responseStatus: 0,
        error: parseError.message || ERROR_MESSAGES.YAML_PARSE_ERROR,
        timestamp: Date.now()
      }]
    };
  }
};