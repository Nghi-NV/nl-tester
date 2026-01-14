import { useState, useCallback, useRef, useEffect } from 'react';
import jsyaml from 'js-yaml';
import { highlightYaml } from './highlighter';
import { LINE_HEIGHT, PADDING_TOP } from './constants';

export interface EditorState {
  content: string;
  highlightedHtml: string;
  currentLine: number;
  scrollTop: number;
  error: string | null;
  lineCount: number;
  stepLines: Map<number, string>;
}

export const useEditorState = (initialContent: string = '') => {
  const [content, setContent] = useState(initialContent);
  const [highlightedHtml, setHighlightedHtml] = useState('');
  const [currentLine, setCurrentLine] = useState(0);
  const [scrollTop, setScrollTop] = useState(0);
  const [error, setError] = useState<string | null>(null);

  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const highlightRef = useRef<HTMLDivElement>(null);
  const gutterRef = useRef<HTMLDivElement>(null);

  // Calculate line count
  const lineCount = content.split('\n').length;

  // Detect step lines (lines with "- name:")
  const stepLines = new Map<number, string>();
  const stepRegex = /^\s*-\s*name:\s*["']?(.+?)["']?\s*$/;
  content.split('\n').forEach((line, idx) => {
    const match = line.match(stepRegex);
    if (match) {
      stepLines.set(idx, match[1].trim());
    }
  });

  // Update highlighting when content changes
  useEffect(() => {
    if (!content) {
      setHighlightedHtml('');
      setError(null);
      return;
    }

    setHighlightedHtml(highlightYaml(content));

    // Validate YAML with debounce
    const timer = setTimeout(() => {
      try {
        jsyaml.load(content);
        setError(null);
      } catch (e: any) {
        setError(e.message?.split('\n')[0] || 'Invalid YAML');
      }
    }, 300);

    return () => clearTimeout(timer);
  }, [content]);

  // Sync content from external source
  useEffect(() => {
    if (initialContent !== content) {
      setContent(initialContent);
    }
  }, [initialContent]);

  // Update current line based on cursor position
  const updateCurrentLine = useCallback(() => {
    if (!textareaRef.current) return;
    const ta = textareaRef.current;
    const pos = ta.selectionStart;
    const text = ta.value.substring(0, pos);
    const newLine = (text.match(/\n/g) || []).length;
    setCurrentLine(newLine);
  }, []);

  // Sync scroll across textarea, highlight, and gutter
  const syncScroll = useCallback(() => {
    if (!textareaRef.current) return;
    const newScrollTop = textareaRef.current.scrollTop;
    setScrollTop(newScrollTop);

    if (highlightRef.current) {
      highlightRef.current.scrollTop = newScrollTop;
    }
    if (gutterRef.current) {
      gutterRef.current.scrollTop = newScrollTop;
    }
  }, []);

  // Get current line highlight position
  const getLineHighlightStyle = useCallback((lineIndex: number) => {
    return {
      top: PADDING_TOP + (lineIndex * LINE_HEIGHT) - scrollTop,
      height: LINE_HEIGHT
    };
  }, [scrollTop]);

  return {
    // State
    content,
    setContent,
    highlightedHtml,
    currentLine,
    scrollTop,
    error,
    lineCount,
    stepLines,

    // Refs
    textareaRef,
    highlightRef,
    gutterRef,

    // Actions
    updateCurrentLine,
    syncScroll,
    getLineHighlightStyle,
  };
};
