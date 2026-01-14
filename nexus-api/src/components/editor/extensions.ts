import type { ReactNode } from 'react';
import { Snippet } from '../../types';

// Extension interface for future expandability
export interface EditorExtension {
  id: string;
  name: string;

  // Language support
  languageId?: string;
  fileExtensions?: string[];

  // Syntax highlighting
  highlighter?: (code: string) => string;

  // Autocomplete
  getSuggestions?: (context: SuggestionContext) => Snippet[];

  // Custom actions
  actions?: EditorAction[];

  // Keyboard bindings
  keybindings?: KeyBinding[];

  // Line decorations (like run buttons)
  lineDecorations?: (lineIndex: number, lineContent: string) => LineDecoration | null;
}

export interface SuggestionContext {
  currentWord: string;
  currentLine: string;
  lineIndex: number;
  content: string;
}

export interface EditorAction {
  id: string;
  label: string;
  icon?: string;
  shortcut?: string;
  handler: (editor: EditorContext) => void;
}

export interface KeyBinding {
  key: string;
  ctrlKey?: boolean;
  metaKey?: boolean;
  shiftKey?: boolean;
  altKey?: boolean;
  handler: (editor: EditorContext) => boolean; // return true to prevent default
}

export interface LineDecoration {
  type: 'button' | 'icon' | 'badge';
  content: ReactNode;
  onClick?: () => void;
  tooltip?: string;
}

export interface EditorContext {
  content: string;
  setContent: (content: string) => void;
  currentLine: number;
  selectionStart: number;
  selectionEnd: number;
  insertText: (text: string, offset?: number) => void;
  setCursor: (position: number) => void;
}

// Extension Registry
class ExtensionRegistry {
  private extensions: Map<string, EditorExtension> = new Map();

  register(extension: EditorExtension) {
    this.extensions.set(extension.id, extension);
  }

  unregister(id: string) {
    this.extensions.delete(id);
  }

  get(id: string): EditorExtension | undefined {
    return this.extensions.get(id);
  }

  getAll(): EditorExtension[] {
    return Array.from(this.extensions.values());
  }

  getForFile(filename: string): EditorExtension | undefined {
    const ext = filename.split('.').pop()?.toLowerCase();
    if (!ext) return undefined;

    for (const extension of this.extensions.values()) {
      if (extension.fileExtensions?.includes(ext)) {
        return extension;
      }
    }
    return undefined;
  }
}

export const extensionRegistry = new ExtensionRegistry();

// Default YAML extension will be registered separately
