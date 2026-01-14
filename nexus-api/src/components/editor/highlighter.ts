// YAML Syntax Highlighter using token-based approach
export const highlightYaml = (code: string): string => {
  const tokens: string[] = [];
  const addToken = (html: string): string => {
    tokens.push(html);
    return `\x00${tokens.length - 1}\x00`;
  };

  let html = code
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");

  // Variables {{variable}}
  html = html.replace(/(\{\{[^}]+\}\})/g, (_, v) =>
    addToken(`<span class="hl-var">${v}</span>`)
  );

  // Strings
  html = html.replace(/"([^"]*)"/g, (_, s) => addToken(`<span class="hl-str">"${s}"</span>`));
  html = html.replace(/'([^']*)'/g, (_, s) => addToken(`<span class="hl-str">'${s}'</span>`));

  // Comments
  html = html.replace(/(#.*)$/gm, (_, c) => addToken(`<span class="hl-cmt">${c}</span>`));

  // List dash
  html = html.replace(/^(\s*)(-)(\s)/gm, (_, i, _d, s) =>
    `${i}${addToken('<span class="hl-dash">-</span>')}${s}`
  );

  // HTTP Methods
  html = html.replace(/(:\s*)(GET|POST|PUT|DELETE|PATCH|HEAD|OPTIONS)(\s|$)/gm, (_, c, m, e) =>
    `${c}${addToken(`<span class="hl-method">${m}</span>`)}${e}`
  );

  // Status codes
  html = html.replace(/(:\s*)([2-5]\d{2})(\s|$)/gm, (_, c, code, e) => {
    const n = parseInt(code);
    const cls = n < 300 ? 'hl-ok' : n < 400 ? 'hl-warn' : 'hl-err';
    return `${c}${addToken(`<span class="${cls}">${code}</span>`)}${e}`;
  });

  // Boolean/null
  html = html.replace(/(:\s*)(true|false|null)(\s|$)/gm, (_, c, v, e) =>
    `${c}${addToken(`<span class="hl-bool">${v}</span>`)}${e}`
  );

  // Numbers
  html = html.replace(/(:\s*)(\d+\.?\d*)(\s|$)/gm, (_, c, n, e) =>
    `${c}${addToken(`<span class="hl-num">${n}</span>`)}${e}`
  );

  // URLs
  html = html.replace(/(:\s*)(\/[\w\-\/\{\}\.]+)/g, (_, c, p) =>
    `${c}${addToken(`<span class="hl-url">${p}</span>`)}`
  );

  // Keys
  html = html.replace(/^(\s*)([a-zA-Z_][\w-]*)(:)/gm, (_, i, k, c) =>
    `${i}${addToken(`<span class="hl-key">${k}</span>`)}${c}`
  );

  // Restore tokens
  html = html.replace(/\x00(\d+)\x00/g, (_, idx) => tokens[parseInt(idx)]);

  return html;
};
