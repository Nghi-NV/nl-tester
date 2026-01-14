import { EditorExtension, extensionRegistry } from './extensions';
import { highlightYaml } from './highlighter';
import { editor } from 'monaco-editor';

/**
 * Nexus API Command Completions
 * All available commands with snippets for autocomplete
 */
export interface NexusCommand {
  label: string;
  detail: string;
  insertText: string;
  category: string;
  isConfig?: boolean; // For config section (before steps)
}

// Config/Header completions (in config section)
export const configCommands: NexusCommand[] = [
  { label: 'config', detail: 'Test configuration block', insertText: 'config:\n  baseUrl: ${1:https://api.example.com}\n  timeout: ${2:5000}', category: 'Config', isConfig: true },
  { label: 'config:baseUrl', detail: 'Base URL configuration', insertText: 'config:\n  baseUrl: ${1:https://api.example.com}', category: 'Config', isConfig: true },
  { label: 'config:timeout', detail: 'Request timeout configuration', insertText: 'config:\n  timeout: ${1:5000}', category: 'Config', isConfig: true },
  { label: 'name', detail: 'Flow name', insertText: 'name: ${1:Flow Name}', category: 'Config', isConfig: true },
];

// All Nexus API commands organized by category
export const nexusCommands: NexusCommand[] = [
  // Step Properties
  { label: 'name', detail: 'Step name', insertText: 'name: ${1:Step Name}', category: 'Step Properties' },
  { label: 'method', detail: 'HTTP method', insertText: 'method: ${1|GET,POST,PUT,DELETE,PATCH,HEAD,OPTIONS|}', category: 'Step Properties' },
  { label: 'url', detail: 'Request URL', insertText: 'url: ${1:/api/endpoint}', category: 'Step Properties' },
  { label: 'headers', detail: 'Request headers', insertText: 'headers:\n  Content-Type: application/json', category: 'Step Properties' },
  { label: 'body', detail: 'Request body', insertText: 'body:\n  ${1:key}: ${2:value}', category: 'Step Properties' },
  { label: 'verify', detail: 'Assertions', insertText: 'verify:\n  status: ${1:200}', category: 'Step Properties' },
  { label: 'extract', detail: 'Variable extraction', insertText: 'extract:\n  ${1:token}: body.${2:token}', category: 'Step Properties' },
  { label: 'flow', detail: 'Include another flow', insertText: 'flow: ${1:filename.yaml}', category: 'Step Properties' },
  { label: 'delay', detail: 'Wait time in milliseconds', insertText: 'delay: ${1:1000}', category: 'Step Properties' },

  // Full Step Templates
  { label: 'GET Step', detail: 'Complete GET request step', insertText: '- name: ${1:Get Data}\n  method: GET\n  url: ${2:/api/resource}\n  verify:\n    status: ${3:200}', category: 'Templates' },
  { label: 'POST Step', detail: 'Complete POST request step', insertText: '- name: ${1:Create Resource}\n  method: POST\n  url: ${2:/api/resource}\n  headers:\n    Content-Type: application/json\n  body:\n    ${3:name}: "${4:value}"\n  verify:\n    status: ${5:201}', category: 'Templates' },
  { label: 'PUT Step', detail: 'Complete PUT request step', insertText: '- name: ${1:Update Resource}\n  method: PUT\n  url: ${2:/api/resource/${3:id}}\n  headers:\n    Content-Type: application/json\n  body:\n    ${4:name}: "${5:value}"\n  verify:\n    status: ${6:200}', category: 'Templates' },
  { label: 'DELETE Step', detail: 'Complete DELETE request step', insertText: '- name: ${1:Delete Resource}\n  method: DELETE\n  url: ${2:/api/resource/${3:id}}\n  verify:\n    status: ${4:204}', category: 'Templates' },

  // Auth Headers
  { label: 'Auth Bearer', detail: 'Bearer token authentication header', insertText: 'Authorization: Bearer ${1:{{token}}}', category: 'Auth' },
  { label: 'Auth Basic', detail: 'Basic authentication header', insertText: 'Authorization: Basic ${1:{{credentials}}}', category: 'Auth' },
  { label: 'API Key Header', detail: 'API key header', insertText: 'X-API-Key: ${1:{{api_key}}}', category: 'Auth' },

  // HTTP Headers
  { label: 'Content-Type:json', detail: 'JSON content type header', insertText: 'Content-Type: application/json', category: 'Headers' },
  { label: 'Content-Type:xml', detail: 'XML content type header', insertText: 'Content-Type: application/xml', category: 'Headers' },
  { label: 'Content-Type:form', detail: 'Form data content type header', insertText: 'Content-Type: application/x-www-form-urlencoded', category: 'Headers' },
  { label: 'Content-Type:text', detail: 'Plain text content type header', insertText: 'Content-Type: text/plain', category: 'Headers' },
  { label: 'Accept:json', detail: 'Accept JSON response header', insertText: 'Accept: application/json', category: 'Headers' },
  { label: 'Accept:xml', detail: 'Accept XML response header', insertText: 'Accept: application/xml', category: 'Headers' },
  { label: 'Accept:all', detail: 'Accept all content types header', insertText: 'Accept: */*', category: 'Headers' },
  { label: 'User-Agent', detail: 'User agent header', insertText: 'User-Agent: ${1:Nexus-API/1.0}', category: 'Headers' },
  { label: 'X-Requested-With', detail: 'X-Requested-With header', insertText: 'X-Requested-With: XMLHttpRequest', category: 'Headers' },
  { label: 'X-CSRF-Token', detail: 'CSRF token header', insertText: 'X-CSRF-Token: ${1:{{csrf_token}}}', category: 'Headers' },
  { label: 'X-Request-ID', detail: 'Request ID header', insertText: 'X-Request-ID: ${1:{{request_id}}}', category: 'Headers' },
  { label: 'Cache-Control:no-cache', detail: 'No cache control header', insertText: 'Cache-Control: no-cache', category: 'Headers' },
  { label: 'Cache-Control:no-store', detail: 'No store cache control header', insertText: 'Cache-Control: no-store', category: 'Headers' },
  { label: 'If-Match', detail: 'If-Match conditional header', insertText: 'If-Match: ${1:"*"}', category: 'Headers' },
  { label: 'If-None-Match', detail: 'If-None-Match conditional header', insertText: 'If-None-Match: ${1:"*"}', category: 'Headers' },
  { label: 'If-Modified-Since', detail: 'If-Modified-Since conditional header', insertText: 'If-Modified-Since: ${1:{{date}}}', category: 'Headers' },
  { label: 'ETag', detail: 'ETag header', insertText: 'ETag: ${1:"{{etag}}"}', category: 'Headers' },
  { label: 'Origin', detail: 'Origin header for CORS', insertText: 'Origin: ${1:https://example.com}', category: 'Headers' },
  { label: 'Referer', detail: 'Referer header', insertText: 'Referer: ${1:https://example.com}', category: 'Headers' },

  // Verification
  { label: 'verify:status', detail: 'Verify HTTP status code', insertText: 'status: ${1:200}', category: 'Verification' },
  { label: 'verify:body', detail: 'Verify response body path', insertText: 'body.${1:data.id}: ${2:value}', category: 'Verification' },
  { label: 'verify:header', detail: 'Verify response header', insertText: 'headers.${1:Content-Type}: ${2:application/json}', category: 'Verification' },
  { label: 'verify:time', detail: 'Verify response time', insertText: 'time: ${1|< 1000|}', category: 'Verification' },
  { label: 'verify:block', detail: 'Complete verification block', insertText: 'verify:\n  status: ${1:200}\n  body.${2:success}: ${3:true}', category: 'Verification' },

  // Extract
  { label: 'extract:token', detail: 'Extract token from response', insertText: 'extract:\n  token: body.${1:token}', category: 'Extract' },
  { label: 'extract:id', detail: 'Extract ID from response', insertText: 'extract:\n  id: body.${1:data.id}', category: 'Extract' },
  { label: 'extract:multiple', detail: 'Extract multiple values', insertText: 'extract:\n  ${1:token}: body.${2:token}\n  ${3:userId}: body.${4:user.id}', category: 'Extract' },

  // Mock Data
  { label: 'mock:email', detail: 'Generate mock email', insertText: '{{$mock.email}}', category: 'Mock Data' },
  { label: 'mock:name', detail: 'Generate mock name', insertText: '{{$mock.name}}', category: 'Mock Data' },
  { label: 'mock:uuid', detail: 'Generate mock UUID', insertText: '{{$mock.uuid}}', category: 'Mock Data' },
  { label: 'mock:date', detail: 'Generate mock date', insertText: '{{$mock.date}}', category: 'Mock Data' },
  { label: 'mock:number', detail: 'Generate mock number', insertText: '{{$mock.number}}', category: 'Mock Data' },
];

/**
 * Check if position is in config section (before steps)
 */
export function isInConfigSection(model: editor.ITextModel, lineNumber: number): boolean {
  for (let i = 1; i < lineNumber; i++) {
    const line = model.getLineContent(i).trim();
    // If we find a step (starts with -), we're in steps section
    if (line.startsWith('-')) {
      return false;
    }
  }
  return true;
}

// YAML Extension
export const yamlExtension: EditorExtension = {
  id: 'yaml',
  name: 'YAML',
  languageId: 'yaml',
  fileExtensions: ['yaml', 'yml'],

  highlighter: highlightYaml,

  getSuggestions: (_context) => {
    // This is now handled by Monaco Editor's completion provider
    return [];
  },

  lineDecorations: (_lineIndex, lineContent) => {
    // Check if line is a step definition
    const match = lineContent.match(/^\s*-\s*name:\s*["']?(.+?)["']?\s*$/);
    if (match) {
      return {
        type: 'button',
        content: null, // Will be rendered by component
        tooltip: `Run: ${match[1].trim()}`,
      };
    }
    return null;
  },
};

// Register YAML extension
extensionRegistry.register(yamlExtension);
