import './monacoEnvironment';
import 'monaco-editor/min/vs/editor/editor.main.css';
import { loader } from '@monaco-editor/react';
import * as monaco from 'monaco-editor/esm/vs/editor/editor.api.js';
import 'monaco-editor/esm/vs/basic-languages/yaml/yaml.contribution';
import 'monaco-editor/esm/vs/basic-languages/markdown/markdown.contribution';

loader.config({ monaco });
