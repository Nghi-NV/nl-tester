# 🛠️ Implementation Guide - Quick Wins

Hướng dẫn implement các tính năng Quick Wins cho Nexus API Tester.

---

## 1. Request/Response History

### Mục tiêu
Lưu lại lịch sử các request/response để có thể xem lại và replay.

### Implementation Steps

#### Step 1: Tạo History Store
```typescript
// stores/historyStore.ts
interface RequestHistory {
  id: string;
  timestamp: number;
  method: string;
  url: string;
  requestHeaders?: Record<string, string>;
  requestBody?: any;
  responseStatus: number;
  responseHeaders?: Record<string, string>;
  responseBody?: any;
  responseTime: number;
  testFlowId?: string;
  stepName?: string;
}

interface HistoryStore {
  history: RequestHistory[];
  maxHistorySize: number;
  addHistory: (entry: Omit<RequestHistory, 'id' | 'timestamp'>) => void;
  clearHistory: () => void;
  getHistoryByFlow: (flowId: string) => RequestHistory[];
}
```

#### Step 2: Integrate vào Runner Service
```typescript
// services/runnerService.ts
import { useHistoryStore } from '../stores/historyStore';

// Trong executeStep function, sau khi nhận response:
const historyEntry = {
  method,
  url,
  requestHeaders: headers,
  requestBody: step.body,
  responseStatus: response.status,
  responseHeaders: Object.fromEntries(response.headers.entries()),
  responseBody,
  responseTime: duration,
  testFlowId: fileId,
  stepName: step.name,
};
useHistoryStore.getState().addHistory(historyEntry);
```

#### Step 3: UI Component
```typescript
// components/HistoryPanel.tsx
- Hiển thị danh sách history
- Filter theo flow, method, status
- Click để xem chi tiết
- Replay button
```

### Tech Stack
- IndexedDB hoặc localStorage để persist
- Zustand store cho state management

---

## 2. Export/Import Test Suites

### Mục tiêu
Export/import toàn bộ test suite để chia sẻ và backup.

### Implementation Steps

#### Step 1: Export Function
```typescript
// utils/exportUtils.ts
export const exportTestSuite = () => {
  const fileStore = useFileStore.getState();
  const envStore = useEnvStore.getState();
  
  const exportData = {
    version: '1.0',
    timestamp: Date.now(),
    files: fileStore.files,
    envVars: envStore.envVars,
  };
  
  const blob = new Blob([JSON.stringify(exportData, null, 2)], {
    type: 'application/json',
  });
  
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `nexus-test-suite-${Date.now()}.json`;
  a.click();
};
```

#### Step 2: Import Function
```typescript
// utils/importUtils.ts
export const importTestSuite = async (file: File) => {
  const text = await file.text();
  const data = JSON.parse(text);
  
  // Validate structure
  if (!data.files || !Array.isArray(data.files)) {
    throw new Error('Invalid test suite format');
  }
  
  // Import files
  useFileStore.getState().setFiles(data.files);
  
  // Import env vars if exists
  if (data.envVars) {
    useEnvStore.getState().setEnvVars(data.envVars);
  }
};
```

#### Step 3: UI Integration
```typescript
// components/SettingsModal.tsx
- Export button
- Import button với file picker
- Confirmation dialog khi import
```

---

## 3. YAML Validation & Linting

### Mục tiêu
Real-time validation và linting cho YAML editor.

### Implementation Steps

#### Step 1: YAML Schema Definition
```typescript
// utils/yamlSchema.ts
import { JSONSchema7 } from 'json-schema';

export const testFlowSchema: JSONSchema7 = {
  type: 'object',
  properties: {
    name: { type: 'string' },
    description: { type: 'string' },
    config: {
      type: 'object',
      properties: {
        baseUrl: { type: 'string' },
        timeout: { type: 'number' },
        headers: { type: 'object' },
      },
    },
    steps: {
      type: 'array',
      items: {
        type: 'object',
        properties: {
          name: { type: 'string' },
          method: { 
            type: 'string',
            enum: ['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS'],
          },
          url: { type: 'string' },
          // ... more properties
        },
        required: ['name'],
      },
    },
  },
  required: ['name'],
};
```

#### Step 2: Validation Service
```typescript
// services/validationService.ts
import Ajv from 'ajv';
import jsyaml from 'js-yaml';

export const validateYaml = (yamlContent: string) => {
  try {
    const data = jsyaml.load(yamlContent);
    const ajv = new Ajv();
    const validate = ajv.compile(testFlowSchema);
    const valid = validate(data);
    
    return {
      valid,
      errors: validate.errors || [],
      data,
    };
  } catch (error) {
    return {
      valid: false,
      errors: [{ message: error.message }],
      data: null,
    };
  }
};
```

#### Step 3: Editor Integration
```typescript
// components/Editor.tsx
- Debounced validation on content change
- Show errors inline
- Error markers in editor
- Error panel at bottom
```

### Tech Stack
- `ajv` cho JSON schema validation
- `js-yaml` cho YAML parsing
- Monaco Editor markers cho error highlighting

---

## 4. OAuth 2.0 / JWT Support

### Mục tiêu
Hỗ trợ OAuth 2.0 flows và JWT token management.

### Implementation Steps

#### Step 1: OAuth Service
```typescript
// services/oauthService.ts
export interface OAuthConfig {
  clientId: string;
  clientSecret: string;
  authorizationUrl: string;
  tokenUrl: string;
  redirectUri: string;
  scope: string[];
}

export const getOAuthToken = async (config: OAuthConfig) => {
  // Authorization Code flow
  // 1. Redirect to authorization URL
  // 2. Get authorization code
  // 3. Exchange for access token
  // 4. Store token
};
```

#### Step 2: JWT Token Management
```typescript
// services/jwtService.ts
export const decodeJWT = (token: string) => {
  const parts = token.split('.');
  const payload = JSON.parse(atob(parts[1]));
  return payload;
};

export const isTokenExpired = (token: string) => {
  const payload = decodeJWT(token);
  return payload.exp * 1000 < Date.now();
};

export const refreshToken = async (refreshToken: string, config: OAuthConfig) => {
  // Refresh token logic
};
```

#### Step 3: YAML Integration
```yaml
# Example usage
steps:
  - name: Get OAuth Token
    oauth:
      flow: authorization_code
      clientId: "{{$env.CLIENT_ID}}"
      clientSecret: "{{$env.CLIENT_SECRET}}"
      authorizationUrl: "https://auth.example.com/authorize"
      tokenUrl: "https://auth.example.com/token"
    extract:
      access_token: body.access_token
      refresh_token: body.refresh_token
  
  - name: Use Token
    method: GET
    url: /api/protected
    headers:
      Authorization: Bearer {{access_token}}
```

---

## 5. Secrets Management

### Mục tiêu
Quản lý API keys và secrets an toàn.

### Implementation Steps

#### Step 1: Encrypted Storage
```typescript
// services/encryptionService.ts
import CryptoJS from 'crypto-js';

const ENCRYPTION_KEY = 'user-provided-key'; // Should be from user input

export const encrypt = (text: string): string => {
  return CryptoJS.AES.encrypt(text, ENCRYPTION_KEY).toString();
};

export const decrypt = (encryptedText: string): string => {
  const bytes = CryptoJS.AES.decrypt(encryptedText, ENCRYPTION_KEY);
  return bytes.toString(CryptoJS.enc.Utf8);
};
```

#### Step 2: Secrets Store
```typescript
// stores/secretsStore.ts
interface Secret {
  id: string;
  name: string;
  value: string; // Encrypted
  environment: string;
  createdAt: number;
}

interface SecretsStore {
  secrets: Secret[];
  addSecret: (secret: Omit<Secret, 'id' | 'createdAt'>) => void;
  getSecret: (id: string) => string | null;
  deleteSecret: (id: string) => void;
}
```

#### Step 3: UI Component
```typescript
// components/SecretsModal.tsx
- List secrets
- Add/Edit/Delete secrets
- Mask values in UI
- Environment selector
- Master password prompt
```

### Security Considerations
- Never store encryption key in code
- Use browser's secure storage
- Consider using Web Crypto API
- Warn users about security implications

---

## 6. Keyboard Shortcuts

### Mục tiêu
Thêm keyboard shortcuts để tăng productivity.

### Implementation Steps

#### Step 1: Shortcuts Hook
```typescript
// hooks/useKeyboardShortcuts.ts
import { useEffect } from 'react';

export const useKeyboardShortcuts = () => {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      // Cmd/Ctrl + S: Save
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault();
        // Save logic
      }
      
      // Cmd/Ctrl + R: Run test
      if ((e.metaKey || e.ctrlKey) && e.key === 'r') {
        e.preventDefault();
        // Run test logic
      }
      
      // Cmd/Ctrl + K: Command palette
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        // Open command palette
      }
    };
    
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, []);
};
```

#### Step 2: Command Palette
```typescript
// components/CommandPalette.tsx
- Searchable list of commands
- Keyboard navigation
- Command categories
- Recent commands
```

---

## 7. Dark/Light Theme Toggle

### Mục tiêu
Hỗ trợ light theme cho users.

### Implementation Steps

#### Step 1: Theme Store
```typescript
// stores/themeStore.ts
type Theme = 'dark' | 'light';

interface ThemeStore {
  theme: Theme;
  setTheme: (theme: Theme) => void;
  toggleTheme: () => void;
}

export const useThemeStore = create<ThemeStore>((set) => ({
  theme: (localStorage.getItem('theme') as Theme) || 'dark',
  setTheme: (theme) => {
    localStorage.setItem('theme', theme);
    document.documentElement.setAttribute('data-theme', theme);
    set({ theme });
  },
  toggleTheme: () => {
    const current = useThemeStore.getState().theme;
    useThemeStore.getState().setTheme(current === 'dark' ? 'light' : 'dark');
  },
}));
```

#### Step 2: CSS Variables
```css
/* styles/themes.css */
[data-theme='dark'] {
  --bg-primary: #0f172a;
  --bg-secondary: #1e293b;
  --text-primary: #f1f5f9;
  /* ... */
}

[data-theme='light'] {
  --bg-primary: #ffffff;
  --bg-secondary: #f8fafc;
  --text-primary: #1e293b;
  /* ... */
}
```

#### Step 3: Theme Toggle Button
```typescript
// components/ThemeToggle.tsx
- Toggle button in header
- Smooth transition
- Persist preference
```

---

## 📚 Resources

### Libraries to Consider
- **IndexedDB**: `idb` hoặc `Dexie.js`
- **Encryption**: `crypto-js` hoặc Web Crypto API
- **Validation**: `ajv`, `yup`, hoặc `zod`
- **OAuth**: `oauth-1.0a`, `simple-oauth2`
- **JWT**: `jose` hoặc `jsonwebtoken`

### Best Practices
1. **Incremental Development**: Implement từng feature một, test kỹ trước khi chuyển sang feature tiếp theo
2. **User Feedback**: Lấy feedback sớm và thường xuyên
3. **Documentation**: Document code và API
4. **Testing**: Viết tests cho các features mới
5. **Performance**: Monitor performance impact của mỗi feature

---

**Last Updated**: 2025-01-XX

