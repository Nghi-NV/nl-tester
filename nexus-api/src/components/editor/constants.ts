// Editor Constants
export const LINE_HEIGHT = 20; // Reduced slightly to integer
export const PADDING_TOP = 16;
export const PADDING_HORIZONTAL = 16;
export const GUTTER_WIDTH = 50;
export const FONT_SIZE = 13;
export const FONT_FAMILY = 'Menlo, Monaco, "Courier New", monospace';
export const FONT_WEIGHT = 400; // Added FONT_WEIGHT constant

// Auto-close pairs for typing
export const AUTO_CLOSE_PAIRS: Record<string, string> = {
  '{': '}',
  '[': ']',
  '(': ')',
  '"': '"',
  "'": "'",
};

// Editor CSS styles
export const editorStyles = `
.editor-container {
    font-family: ${FONT_FAMILY};
    font-size: ${FONT_SIZE}px;
    line-height: ${LINE_HEIGHT}px;
}

.hl-key { color: #22d3ee; font-weight: 600; }
.hl-str { color: #6ee7b7; }
.hl-cmt { color: #64748b; font-style: italic; }
.hl-var { color: #fbbf24; font-weight: bold; }
.hl-dash { color: #f472b6; font-weight: bold; }
.hl-method { color: #fb7185; font-weight: bold; }
.hl-ok { color: #34d399; font-weight: bold; }
.hl-warn { color: #facc15; font-weight: bold; }
.hl-err { color: #f87171; font-weight: bold; }
.hl-bool { color: #c084fc; font-weight: 600; }
.hl-num { color: #fdba74; }
.hl-url { color: #60a5fa; }

.editor-textarea {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    /* Container handles scrolling */
    flex: 1;
    position: relative;
    overflow: auto; 
    background: transparent;
    min-height: 0; /* Critical for flex scrolling */
}

/* Custom Scrollbar for container */
.editor-scroll-container::-webkit-scrollbar {
    width: 10px; /* Slightly wider for ease of use */
    height: 10px;
}

.editor-scroll-container::-webkit-scrollbar-track {
    background: transparent;
}

.editor-scroll-container::-webkit-scrollbar-thumb {
    background: #475569;
    border-radius: 5px;
    border: 2px solid #0f172a; /* Padding effect */
}

.editor-scroll-container::-webkit-scrollbar-thumb:hover {
    background: #64748b;
}

.editor-scroll-container::-webkit-scrollbar-corner {
    background: transparent;
}

.editor-content-wrapper {
    display: grid;
    min-width: 100%;
    min-height: 100%;
    width: max-content; /* Critical: allows scrolling for long lines */
    position: relative;
}

/* Textarea is overlay for input capturing */
.editor-textarea {
    grid-area: 1 / 1;
    display: block;
    position: relative; /* In grid, we don't need absolute */
    width: 100%;
    height: 100%;
    padding: ${PADDING_TOP}px ${PADDING_HORIZONTAL}px 100px ${PADDING_HORIZONTAL}px;
    margin: 0;
    background: transparent;
    color: transparent;
    caret-color: #22d3ee;
    font-family: ${FONT_FAMILY} !important;
    font-size: ${FONT_SIZE}px !important;
    font-weight: 400 !important;
    line-height: ${LINE_HEIGHT}px !important;
    resize: none;
    outline: none;
    border: none;
    overflow: hidden !important;
    white-space: pre !important;
    tab-size: 2 !important;
    -moz-tab-size: 2 !important;
    word-spacing: normal !important;
    letter-spacing: normal !important;
    font-variant-ligatures: none !important;
    -webkit-font-smoothing: antialiased;
    z-index: 1; /* Input layer */
}

.editor-textarea::selection {
    background: rgba(34, 211, 238, 0.3);
}

/* Highlight drives the dimension of the wrapper */
.editor-highlight {
    grid-area: 1 / 1;
    position: relative; 
    padding: ${PADDING_TOP}px ${PADDING_HORIZONTAL}px 100px ${PADDING_HORIZONTAL}px;
    margin: 0;
    overflow: visible;
    white-space: pre !important;
    pointer-events: none;
    font-family: ${FONT_FAMILY} !important;
    font-size: ${FONT_SIZE}px !important;
    font-weight: 400 !important;
    line-height: ${LINE_HEIGHT}px !important;
    color: #e2e8f0;
    tab-size: 2 !important;
    -moz-tab-size: 2 !important;
    word-spacing: normal !important;
    letter-spacing: normal !important;
    font-variant-ligatures: none !important;
    -webkit-font-smoothing: antialiased;
    z-index: 0; /* Visual layer */
}

.editor-highlight code,
.editor-highlight span {
    font-family: ${FONT_FAMILY} !important;
    font-size: ${FONT_SIZE}px !important;
    font-weight: 400 !important;
    line-height: ${LINE_HEIGHT}px !important;
    tab-size: 2 !important;
    word-spacing: normal !important;
    letter-spacing: normal !important;
    font-variant-ligatures: none !important;
}

.editor-highlight code {
    display: inline-block;
    min-width: max-content;
    padding-right: ${PADDING_HORIZONTAL}px;
}

.editor-gutter {
    width: ${GUTTER_WIDTH}px;
    padding-top: ${PADDING_TOP}px;
    background: rgba(15, 23, 42, 0.7);
    border-right: 1px solid #1e293b;
    user-select: none;
    overflow: hidden;
}

.gutter-line {
    height: ${LINE_HEIGHT}px;
    display: flex;
    align-items: center;
    font-size: 11px;
    padding-right: 8px;
    white-space: nowrap;
}
`;
