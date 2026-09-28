import { invoke } from '@tauri-apps/api/core';
import { GoogleGenAI } from '@google/genai';
import { AiConfig, FileNode } from '../types';
import { findFileById, getAllDescendantFiles } from '../stores';
import { AI_CONFIG, APP_CONFIG, ERROR_MESSAGES, FILE_CONFIG } from '../constants';

interface AiContextFile {
  path: string;
  content: string;
}

const getMentionedFiles = (prompt: string, files: FileNode[]): AiContextFile[] => {
  const mentions = [...prompt.matchAll(/@"([^"]+)"|@([^\s]+)/g)].map(match => match[1] ?? match[2]);
  const allFiles = files.flatMap(file => getAllDescendantFiles(file));
  const selected = new Map<string, AiContextFile>();

  for (const mention of mentions) {
    const reference = mention.replace(/\\/g, '/').replace(/[),.;!?]+$/, '');
    const file = allFiles.find(candidate => {
      const normalizedPath = candidate.id.replace(/\\/g, '/');
      const normalizedName = candidate.name.replace(/\\/g, '/');
      return normalizedName === reference
        || normalizedName === `${reference}${FILE_CONFIG.YAML_EXTENSION}`
        || normalizedPath.endsWith(`/${reference}`)
        || normalizedPath.endsWith(`/${reference}${FILE_CONFIG.YAML_EXTENSION}`);
    });
    if (file?.content !== undefined) selected.set(file.id, { path: file.id, content: file.content });
  }
  return [...selected.values()];
};

export const generateAiResponse = async (
  prompt: string,
  config: AiConfig,
  files: FileNode[],
  workspaceRoot: string | null,
): Promise<string> => {
  const contextFiles = getMentionedFiles(prompt, files);
  const context = contextFiles.map(file => `\n--- FILE: ${file.path} ---\n${file.content}\n--- END FILE ---\n`).join('');

  if (config.provider !== 'gemini') {
    if (!workspaceRoot) throw new Error('Open a workspace before using a local AI provider.');
    return invoke<string>('generate_ai_response', {
      provider: config.provider,
      binaryPath: config.binaryPath || null,
      workspacePath: workspaceRoot,
      prompt,
      contextFiles,
    });
  }

  if (!config.apiKey) {
    await new Promise(resolve => setTimeout(resolve, APP_CONFIG.DEBUG_MODE_DELAY));
    return `Configure a Gemini API key in AI settings to use Gemini.\n\n${contextFiles.length > 0 ? `Included ${contextFiles.length} referenced file(s).` : 'No files were added as context.'}`;
  }

  try {
    const ai = new GoogleGenAI({ apiKey: config.apiKey });
    const response = await ai.models.generateContent({
      model: config.model || AI_CONFIG.DEFAULT_MODEL,
      contents: `${context}\n\nUser request: ${prompt}`,
      config: { systemInstruction: AI_CONFIG.SYSTEM_INSTRUCTION },
    });
    return response.text || ERROR_MESSAGES.AI_NO_RESPONSE;
  } catch (error: any) {
    console.error('AI Error:', error);
    return ERROR_MESSAGES.AI_ERROR(error.message || 'Unknown error');
  }
};

export const getActiveFileContent = (files: FileNode[], path: string | null): string | null => {
  if (!path) return null;
  return findFileById(files, path)?.content ?? null;
};
