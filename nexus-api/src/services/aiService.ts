import { GoogleGenAI } from "@google/genai";
import { AiConfig, FileNode } from "../types";
import { getAllDescendantFiles } from "../stores";
import { AI_CONFIG, APP_CONFIG, ERROR_MESSAGES, FILE_CONFIG } from "../constants";

export const generateAiResponse = async (
  prompt: string,
  config: AiConfig,
  files: FileNode[]
): Promise<string> => {

  // 1. Resolve Mentions (@filename)
  let context = "";
  const mentions = prompt.match(AI_CONFIG.MENTION_REGEX);

  if (mentions) {
    const allFiles = files.flatMap(f => getAllDescendantFiles(f));

    for (const mention of mentions) {
      const filename = mention.substring(1); // remove @
      const file = allFiles.find(
        f => f.name === filename || f.name === filename + FILE_CONFIG.YAML_EXTENSION
      );

      if (file && file.content) {
        context += `\n--- FILE: ${file.name} ---\n${file.content}\n--- END FILE ---\n`;
      }
    }
  }

  const fullPrompt = `${context}\n\nUser Question: ${prompt}`;

  // 2. Debug Mode (No API Key)
  if (!config.apiKey) {
    await new Promise(r => setTimeout(r, APP_CONFIG.DEBUG_MODE_DELAY));
    return `**Debug Mode (No API Key)**\n\nI received your request:\n"${prompt}"\n\n${context ? `I also found context from ${mentions?.length} files.` : ''}\n\nTo get real AI responses, please configure a valid Google Gemini API Key in the settings menu above. For now, here is a mock YAML snippet:\n\n\`\`\`yaml\nname: Debug Test\nsteps:\n  - name: Mock Step\n    method: GET\n    url: /debug\n\`\`\``;
  }

  // 3. Real API Call
  try {
    const ai = new GoogleGenAI({ apiKey: config.apiKey });
    const modelName = config.model || AI_CONFIG.DEFAULT_MODEL;

    const response = await ai.models.generateContent({
      model: modelName,
      contents: fullPrompt,
      config: {
        systemInstruction: AI_CONFIG.SYSTEM_INSTRUCTION,
      }
    });

    return response.text || ERROR_MESSAGES.AI_NO_RESPONSE;

  } catch (error: any) {
    console.error("AI Error:", error);
    return ERROR_MESSAGES.AI_ERROR(error.message || 'Unknown error');
  }
};