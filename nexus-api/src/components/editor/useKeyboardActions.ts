import React, { useCallback } from 'react';
import { AUTO_CLOSE_PAIRS } from './constants';

interface UseKeyboardActionsProps {
  textareaRef: React.RefObject<HTMLTextAreaElement>;
  content: string;
  setContent: (content: string) => void;
  updateCurrentLine: () => void;
  onSuggestionSelect?: () => void;
}

export const useKeyboardActions = ({
  textareaRef,
  content,
  setContent,
  updateCurrentLine,
}: UseKeyboardActionsProps) => {

  // Insert text at cursor position
  const insertText = useCallback((text: string, cursorOffset = text.length) => {
    const ta = textareaRef.current;
    if (!ta) return;

    const start = ta.selectionStart;
    const end = ta.selectionEnd;
    const newValue = content.substring(0, start) + text + content.substring(end);
    setContent(newValue);

    setTimeout(() => {
      ta.focus();
      const newPos = start + cursorOffset;
      ta.setSelectionRange(newPos, newPos);
      updateCurrentLine();
    }, 0);
  }, [content, setContent, textareaRef, updateCurrentLine]);

  // Insert auto-close pair
  const insertPair = useCallback((open: string, close: string) => {
    const ta = textareaRef.current;
    if (!ta) return;

    const start = ta.selectionStart;
    const end = ta.selectionEnd;
    const selected = content.substring(start, end);

    if (selected) {
      // Wrap selection
      const newValue = content.substring(0, start) + open + selected + close + content.substring(end);
      setContent(newValue);
      setTimeout(() => {
        ta.focus();
        ta.setSelectionRange(start + 1, end + 1);
      }, 0);
    } else {
      // Insert pair with cursor in middle
      const newValue = content.substring(0, start) + open + close + content.substring(end);
      setContent(newValue);
      setTimeout(() => {
        ta.focus();
        ta.setSelectionRange(start + 1, start + 1);
      }, 0);
    }
  }, [content, setContent, textareaRef]);

  // Duplicate current line (Cmd/Ctrl + D)
  const duplicateLine = useCallback(() => {
    const ta = textareaRef.current;
    if (!ta) return;

    const pos = ta.selectionStart;
    const lineStart = content.lastIndexOf('\n', pos - 1) + 1;
    const lineEnd = content.indexOf('\n', pos);
    const end = lineEnd === -1 ? content.length : lineEnd;
    const line = content.substring(lineStart, end);

    const newValue = content.substring(0, end) + '\n' + line + content.substring(end);
    setContent(newValue);

    setTimeout(() => {
      ta.focus();
      const newPos = end + 1 + (pos - lineStart);
      ta.setSelectionRange(newPos, newPos);
      updateCurrentLine();
    }, 0);
  }, [content, setContent, textareaRef, updateCurrentLine]);

  // Toggle comment (Cmd/Ctrl + /)
  const toggleComment = useCallback(() => {
    const ta = textareaRef.current;
    if (!ta) return;

    const start = ta.selectionStart;
    const end = ta.selectionEnd;
    const lineStart = content.lastIndexOf('\n', start - 1) + 1;
    const lineEnd = content.indexOf('\n', end);
    const actualEnd = lineEnd === -1 ? content.length : lineEnd;

    const lines = content.substring(lineStart, actualEnd).split('\n');
    const allCommented = lines.every(l => l.trimStart().startsWith('#'));

    const modified = lines.map(line => {
      if (allCommented) {
        return line.replace(/^(\s*)#\s?/, '$1');
      } else {
        const indent = line.match(/^(\s*)/)?.[0] || '';
        return indent + '# ' + line.trimStart();
      }
    });

    const newValue = content.substring(0, lineStart) + modified.join('\n') + content.substring(actualEnd);
    setContent(newValue);

    setTimeout(() => {
      ta.focus();
      ta.setSelectionRange(start, start);
      updateCurrentLine();
    }, 0);
  }, [content, setContent, textareaRef, updateCurrentLine]);

  // Indent/dedent block (Tab / Shift+Tab)
  const handleIndent = useCallback((indent: boolean) => {
    const ta = textareaRef.current;
    if (!ta) return;

    const start = ta.selectionStart;
    const end = ta.selectionEnd;

    // Single cursor - just insert spaces
    if (start === end && indent) {
      insertText('  ', 2);
      return;
    }

    // Block selection
    const lineStart = content.lastIndexOf('\n', start - 1) + 1;
    const lineEnd = content.indexOf('\n', end - 1);
    const actualEnd = lineEnd === -1 ? content.length : lineEnd;

    const lines = content.substring(lineStart, actualEnd).split('\n');
    const modified = lines.map(line =>
      indent ? '  ' + line : line.replace(/^  /, '')
    );

    const newValue = content.substring(0, lineStart) + modified.join('\n') + content.substring(actualEnd);
    const diff = modified.join('\n').length - lines.join('\n').length;

    setContent(newValue);

    setTimeout(() => {
      ta.focus();
      ta.setSelectionRange(start, end + diff);
      updateCurrentLine();
    }, 0);
  }, [content, setContent, textareaRef, insertText, updateCurrentLine]);

  // Auto-indent on Enter
  const handleEnter = useCallback(() => {
    const ta = textareaRef.current;
    if (!ta) return;

    const line = content.substring(0, ta.selectionStart).split('\n').pop() || '';
    const indent = line.match(/^(\s*)/)?.[0] || '';
    const extra = line.trim().endsWith(':') ? '  ' : '';
    insertText(`\n${indent}${extra}`);
  }, [content, insertText, textareaRef]);

  // Handle backspace for deleting pairs
  const handleBackspace = useCallback((): boolean => {
    const ta = textareaRef.current;
    if (!ta) return false;

    const pos = ta.selectionStart;
    if (ta.selectionStart !== ta.selectionEnd) return false;

    const before = content[pos - 1];
    const after = content[pos];

    if (AUTO_CLOSE_PAIRS[before] === after) {
      const newValue = content.substring(0, pos - 1) + content.substring(pos + 1);
      setContent(newValue);
      setTimeout(() => {
        ta.focus();
        ta.setSelectionRange(pos - 1, pos - 1);
      }, 0);
      return true;
    }
    return false;
  }, [content, setContent, textareaRef]);

  return {
    insertText,
    insertPair,
    duplicateLine,
    toggleComment,
    handleIndent,
    handleEnter,
    handleBackspace,
  };
};
