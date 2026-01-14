// Basic extension registry for Editor
export interface EditorExtension {
  id: string;
  name: string;
  languageId: string;
  fileExtensions: string[];
  highlighter?: any;
  getSuggestions?: (context: any) => any[];
  lineDecorations?: (lineIndex: number, lineContent: string) => any;
  activate?: (registry: any) => void;
}

class ExtensionRegistry {
  private extensions: EditorExtension[] = [];

  register(extension: EditorExtension) {
    this.extensions.push(extension);
  }

  getExtensions() {
    return this.extensions;
  }
}

export const extensionRegistry = new ExtensionRegistry();
