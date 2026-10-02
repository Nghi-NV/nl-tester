import React, { useEffect, useId, useMemo, useState } from 'react';
import ReactMarkdown, { Components } from 'react-markdown';
import remarkGfm from 'remark-gfm';
import mermaid from 'mermaid';

interface FilePreviewProps {
    filename: string;
    content: string;
}

const extensionFor = (filename: string) => filename.split('.').pop()?.toLowerCase() ?? '';

const htmlPreviewDocument = (content: string) => {
    const securityPolicy = '<meta http-equiv="Content-Security-Policy" content="default-src \'none\'; img-src data: blob:; media-src data: blob:; style-src \'unsafe-inline\'; font-src data:; script-src \'none\'; connect-src \'none\'; frame-src \'none\'; object-src \'none\';">';
    if (/<head(?:\s[^>]*)?>/i.test(content)) {
        return content.replace(/<head(?:\s[^>]*)?>/i, match => `${match}${securityPolicy}`);
    }
    if (/<html(?:\s[^>]*)?>/i.test(content)) {
        return content.replace(/<html(?:\s[^>]*)?>/i, match => `${match}<head>${securityPolicy}</head>`);
    }
    return `<!doctype html><html><head>${securityPolicy}</head><body>${content}</body></html>`;
};

const parseCsv = (source: string) => {
    const rows: string[][] = [];
    let row: string[] = [];
    let cell = '';
    let quoted = false;

    for (let index = 0; index < source.length; index += 1) {
        const character = source[index];
        if (quoted && character === '"' && source[index + 1] === '"') {
            cell += '"';
            index += 1;
        } else if (character === '"') {
            quoted = !quoted;
        } else if (!quoted && character === ',') {
            row.push(cell);
            cell = '';
        } else if (!quoted && (character === '\n' || character === '\r')) {
            if (character === '\r' && source[index + 1] === '\n') index += 1;
            row.push(cell);
            if (row.some(value => value.length > 0)) rows.push(row);
            row = [];
            cell = '';
        } else {
            cell += character;
        }
    }

    row.push(cell);
    if (row.some(value => value.length > 0)) rows.push(row);
    return rows;
};

let mermaidInitialized = false;

const MermaidDiagram: React.FC<{ source: string }> = ({ source }) => {
    const reactId = useId();
    const diagramId = `lumi-mermaid-${reactId.replace(/[^a-zA-Z0-9_-]/g, '')}`;
    const [svg, setSvg] = useState('');
    const [error, setError] = useState<string | null>(null);

    useEffect(() => {
        let cancelled = false;
        setSvg('');
        setError(null);
        if (!mermaidInitialized) {
            mermaid.initialize({ startOnLoad: false, securityLevel: 'strict', theme: 'dark' });
            mermaidInitialized = true;
        }

        void mermaid.render(diagramId, source.trim())
            .then(result => {
                if (!cancelled) setSvg(result.svg);
            })
            .catch(renderError => {
                if (!cancelled) setError(String(renderError));
            });
        return () => { cancelled = true; };
    }, [diagramId, source]);

    if (error) {
        return <div className="ide-preview-error" role="alert">Mermaid preview failed: {error}</div>;
    }
    if (!svg) return <div className="ide-preview-loading" role="status">Rendering diagram…</div>;
    return <div className="ide-mermaid-diagram" role="img" aria-label="Mermaid diagram" dangerouslySetInnerHTML={{ __html: svg }} />;
};

const markdownComponents: Components = {
    pre({ children, ...props }) {
        const child = React.Children.toArray(children)[0];
        if (React.isValidElement(child)) {
            const childProps = child.props as { className?: string; children?: React.ReactNode };
            if (childProps.className?.split(/\s+/).includes('language-mermaid')) {
                return <MermaidDiagram source={String(childProps.children ?? '')} />;
            }
        }
        return <pre {...props}>{children}</pre>;
    },
    a({ href, children, ...props }) {
        return <a {...props} href={href} target="_blank" rel="noopener noreferrer">{children}</a>;
    },
};

export const FilePreview: React.FC<FilePreviewProps> = ({ filename, content }) => {
    const extension = extensionFor(filename);
    const htmlDocument = useMemo(() => htmlPreviewDocument(content), [content]);
    const csvRows = useMemo(() => extension === 'csv' ? parseCsv(content) : [], [content, extension]);
    const formattedJson = useMemo(() => {
        if (extension !== 'json') return null;
        try {
            return JSON.stringify(JSON.parse(content), null, 2);
        } catch (error) {
            return `Invalid JSON: ${String(error)}\n\n${content}`;
        }
    }, [content, extension]);

    if (extension === 'html' || extension === 'htm') {
        return (
            <div className="ide-file-preview ide-html-preview">
                <iframe
                    title={`Preview of ${filename}`}
                    sandbox=""
                    referrerPolicy="no-referrer"
                    srcDoc={htmlDocument}
                />
            </div>
        );
    }

    if (extension === 'md' || extension === 'markdown') {
        return (
            <div className="ide-file-preview ide-markdown-preview">
                <article className="ide-markdown-content">
                    <ReactMarkdown remarkPlugins={[remarkGfm]} components={markdownComponents}>{content}</ReactMarkdown>
                </article>
            </div>
        );
    }

    if (extension === 'mmd' || extension === 'mermaid') {
        return (
            <div className="ide-file-preview ide-diagram-preview">
                <MermaidDiagram source={content} />
            </div>
        );
    }

    if (extension === 'svg') {
        return (
            <div className="ide-file-preview ide-image-preview">
                <img src={`data:image/svg+xml;charset=utf-8,${encodeURIComponent(content)}`} alt={filename} />
            </div>
        );
    }

    if (extension === 'csv') {
        return (
            <div className="ide-file-preview ide-csv-preview">
                <table>
                    <thead><tr>{(csvRows[0] ?? []).map((cell, index) => <th key={index}>{cell}</th>)}</tr></thead>
                    <tbody>{csvRows.slice(1).map((row, rowIndex) => (
                        <tr key={rowIndex}>{row.map((cell, cellIndex) => <td key={cellIndex}>{cell}</td>)}</tr>
                    ))}</tbody>
                </table>
                {csvRows.length === 0 && <p>This CSV file is empty.</p>}
            </div>
        );
    }

    if (formattedJson !== null) {
        return (
            <div className="ide-file-preview ide-json-preview">
                <pre><code>{formattedJson}</code></pre>
            </div>
        );
    }

    return <div className="ide-file-preview">Preview is not available for {filename}.</div>;
};
