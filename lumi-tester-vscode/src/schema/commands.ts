// Lumi Tester Command Definitions
// Auto-generated from lumi-tester/docs/commands.md

export interface CommandParam {
  name: string;
  type: 'string' | 'number' | 'boolean' | 'object';
  description: string;
  required?: boolean;
  snippet?: string;
}

export interface LumiCommand {
  name: string;
  aliases?: string[];
  category: string;
  description: string;
  hasParams: boolean;
  snippet?: string;
  params?: CommandParam[];
  platforms?: string[];
}

export const LUMI_COMMANDS: LumiCommand[] = [
  // App Management
  {
    name: 'launchApp',
    aliases: ['open'],
    category: 'App Management',
    description: 'Launch an application',
    hasParams: true,
    snippet: 'launchApp:\n    appId: "$1"',
    params: [
      { name: 'appId', type: 'string', description: 'Package name (Android) or Bundle ID (iOS)' },
      { name: 'clearState', type: 'boolean', description: 'Clear app data before launch' },
      { name: 'clearKeychain', type: 'boolean', description: 'Clear iOS Keychain (simulator only)' },
      { name: 'stopApp', type: 'boolean', description: 'Stop app before launch (default: true)' },
      {
        name: 'permissions',
        type: 'object',
        description: 'Permissions to set',
        snippet: 'permissions:\n    ${1:android.permission.CAMERA}: "${2|allow,deny|}"'
      },
      { name: 'label', type: 'string', description: 'Optional label for custom logging' }
    ]
  },
  {
    name: 'stopApp',
    category: 'App Management',
    description: 'Stop the current application',
    hasParams: false
  },
  {
    name: 'clearAppData',
    category: 'App Management',
    description: 'Clear application data (reset)',
    hasParams: true,
    snippet: 'clearAppData: "$1"'
  },
  {
    name: 'installApp',
    category: 'App Management',
    description: 'Install an APK file',
    hasParams: true,
    snippet: 'installApp: "$1"'
  },
  {
    name: 'uninstallApp',
    category: 'App Management',
    description: 'Uninstall an application',
    hasParams: true,
    snippet: 'uninstallApp: "$1"'
  },
  {
    name: 'backgroundApp',
    category: 'App Management',
    description: 'Put app in background for a duration',
    hasParams: true,
    snippet: 'backgroundApp:\n    durationMs: ${1:5000}'
  },
  {
    name: 'selectDisplay',
    aliases: ['display'],
    category: 'App Management',
    description: 'Select display for interaction (Android Auto)',
    hasParams: true,
    snippet: 'selectDisplay: "${1:0}"'
  },

  // Interaction
  {
    name: 'tap',
    aliases: ['tapOn'],
    category: 'Interaction',
    description: 'Tap on an element',
    hasParams: true,
    snippet: 'tap:\n    ${1|text,id,css,xpath,point|}: "$2"',
    params: [
      { name: 'text', type: 'string', description: 'Find by exact text' },
      { name: 'id', type: 'string', description: 'Find by resource ID' },
      { name: 'css', type: 'string', description: 'Find by CSS selector (Web only)' },
      { name: 'xpath', type: 'string', description: 'Find by XPath' },
      { name: 'point', type: 'string', description: 'Tap coordinates (x,y or x%,y%)' },
      { name: 'regex', type: 'string', description: 'Find by regex pattern' },
      { name: 'index', type: 'number', description: 'Element index (0-based)' },
      { name: 'type', type: 'string', description: 'Element type (Button, EditText...)' },
      { name: 'placeholder', type: 'string', description: 'Find by placeholder text' },
      { name: 'role', type: 'string', description: 'Find by role attribute' },
      { name: 'image', type: 'string', description: 'Find by image template matching' },
      { name: 'optional', type: 'boolean', description: 'Skip if not found' },
      { name: 'desc', type: 'string', description: 'Find by content description/accessibility ID' },
      { name: 'exact', type: 'boolean', description: 'Match text exactly (case-sensitive)' },
      { name: 'retryTapIfNoChange', type: 'boolean', description: 'Retry tap if UI does not change' },
      {
        name: 'scrollable',
        type: 'object',
        description: 'Auto-scroll configuration',
        snippet: 'scrollable:\n    index: ${1:0}\n    itemIndex: ${2:0}'
      },
      // Relative positioning
      {
        name: 'rightOf',
        type: 'string',
        description: 'Find element right of anchor',
        snippet: 'rightOf:\n    text: "${1:text}"'
      },
      {
        name: 'leftOf',
        type: 'string',
        description: 'Find element left of anchor',
        snippet: 'leftOf:\n    text: "${1:text}"'
      },
      {
        name: 'above',
        type: 'string',
        description: 'Find element above anchor',
        snippet: 'above:\n    text: "${1:text}"'
      },
      {
        name: 'below',
        type: 'string',
        description: 'Find element below anchor',
        snippet: 'below:\n    text: "${1:text}"'
      },
      { name: 'label', type: 'string', description: 'Optional label for custom logging' },
      {
        name: 'ocr',
        type: 'object',
        description: 'Find by OCR',
        snippet: 'ocr:\n    text: "${1:text_to_find}"\n    region: "${2|all,top-half,bottom-half,left-half,right-half,center|}"'
      }
    ]
  },
  {
    name: 'doubleTap',
    category: 'Interaction',
    description: 'Double tap on an element',
    hasParams: true,
    snippet: 'doubleTap:\n    ${1|text,id,css,xpath,point|}: "$2"',
    params: [
      { name: 'text', type: 'string', description: 'Find by exact text' },
      { name: 'id', type: 'string', description: 'Find by resource ID' },
      { name: 'css', type: 'string', description: 'Find by CSS selector (Web only)' },
      { name: 'xpath', type: 'string', description: 'Find by XPath' },
      { name: 'point', type: 'string', description: 'Tap coordinates (x,y or x%,y%)' },
      { name: 'regex', type: 'string', description: 'Find by regex pattern' },
      { name: 'index', type: 'number', description: 'Element index (0-based)' },
      { name: 'type', type: 'string', description: 'Element type (Button, EditText...)' },
      { name: 'placeholder', type: 'string', description: 'Find by placeholder text' },
      { name: 'role', type: 'string', description: 'Find by role attribute' },
      { name: 'image', type: 'string', description: 'Find by image template matching' },
      { name: 'optional', type: 'boolean', description: 'Skip if not found' },
      { name: 'desc', type: 'string', description: 'Find by content description/accessibility ID' },
      { name: 'exact', type: 'boolean', description: 'Match text exactly (case-sensitive)' },
      { name: 'retryTapIfNoChange', type: 'boolean', description: 'Retry tap if UI does not change' },
      {
        name: 'scrollable',
        type: 'object',
        description: 'Auto-scroll configuration',
        snippet: 'scrollable:\n    index: ${1:0}\n    itemIndex: ${2:0}'
      },
      // Relative positioning
      {
        name: 'rightOf',
        type: 'string',
        description: 'Find element right of anchor',
        snippet: 'rightOf:\n    text: "${1:text}"'
      },
      {
        name: 'leftOf',
        type: 'string',
        description: 'Find element left of anchor',
        snippet: 'leftOf:\n    text: "${1:text}"'
      },
      {
        name: 'above',
        type: 'string',
        description: 'Find element above anchor',
        snippet: 'above:\n    text: "${1:text}"'
      },
      {
        name: 'below',
        type: 'string',
        description: 'Find element below anchor',
        snippet: 'below:\n    text: "${1:text}"'
      },
      { name: 'label', type: 'string', description: 'Optional label for custom logging' },
      {
        name: 'ocr',
        type: 'object',
        description: 'Find by OCR',
        snippet: 'ocr:\n    text: "${1:text_to_find}"\n    region: "${2|all,top-half,bottom-half,left-half,right-half,center|}"'
      }
    ]
  },
  {
    name: 'longPress',
    category: 'Interaction',
    description: 'Long press on an element (1000ms)',
    hasParams: true,
    snippet: 'longPress:\n    ${1|text,id,css,xpath,point|}: "$2"',
    params: [
      { name: 'text', type: 'string', description: 'Find by exact text' },
      { name: 'id', type: 'string', description: 'Find by resource ID' },
      { name: 'css', type: 'string', description: 'Find by CSS selector (Web only)' },
      { name: 'xpath', type: 'string', description: 'Find by XPath' },
      { name: 'point', type: 'string', description: 'Press coordinates (x,y or x%,y%)' },
      { name: 'regex', type: 'string', description: 'Find by regex pattern' },
      { name: 'index', type: 'number', description: 'Element index (0-based)' },
      { name: 'type', type: 'string', description: 'Element type (Button, EditText...)' },
      { name: 'placeholder', type: 'string', description: 'Find by placeholder text' },
      { name: 'role', type: 'string', description: 'Find by role attribute' },
      { name: 'image', type: 'string', description: 'Find by image template matching' },
      { name: 'optional', type: 'boolean', description: 'Skip if not found' },
      { name: 'desc', type: 'string', description: 'Find by content description/accessibility ID' },
      { name: 'exact', type: 'boolean', description: 'Match text exactly (case-sensitive)' },
      { name: 'retryTapIfNoChange', type: 'boolean', description: 'Retry tap if UI does not change' },
      {
        name: 'scrollable',
        type: 'object',
        description: 'Auto-scroll configuration',
        snippet: 'scrollable:\n    index: ${1:0}\n    itemIndex: ${2:0}'
      },
      // Relative positioning
      {
        name: 'rightOf',
        type: 'string',
        description: 'Find element right of anchor',
        snippet: 'rightOf:\n    text: "${1:text}"'
      },
      {
        name: 'leftOf',
        type: 'string',
        description: 'Find element left of anchor',
        snippet: 'leftOf:\n    text: "${1:text}"'
      },
      {
        name: 'above',
        type: 'string',
        description: 'Find element above anchor',
        snippet: 'above:\n    text: "${1:text}"'
      },
      {
        name: 'below',
        type: 'string',
        description: 'Find element below anchor',
        snippet: 'below:\n    text: "${1:text}"'
      },
      { name: 'label', type: 'string', description: 'Optional label for custom logging' },
      {
        name: 'ocr',
        type: 'object',
        description: 'Find by OCR',
        snippet: 'ocr:\n    text: "${1:text_to_find}"\n    region: "${2|all,top-half,bottom-half,left-half,right-half,center|}"'
      }
    ]
  },
  {
    name: 'rightClick',
    aliases: ['contextClick'],
    category: 'Interaction',
    description: 'Right click on an element (Web/Desktop)',
    hasParams: true,
    snippet: 'rightClick:\n    ${1|text,id,css,xpath|}: "$2"',
    params: [
      { name: 'text', type: 'string', description: 'Find by exact text' },
      { name: 'id', type: 'string', description: 'Find by resource ID' },
      { name: 'css', type: 'string', description: 'Find by CSS selector (Web only)' },
      { name: 'xpath', type: 'string', description: 'Find by XPath' },
      { name: 'point', type: 'string', description: 'Tap coordinates (x,y or x%,y%)' },
      { name: 'regex', type: 'string', description: 'Find by regex pattern' },
      { name: 'index', type: 'number', description: 'Element index (0-based)' },
      { name: 'type', type: 'string', description: 'Element type (Button, EditText...)' },
      { name: 'placeholder', type: 'string', description: 'Find by placeholder text' },
      { name: 'role', type: 'string', description: 'Find by role attribute' },
      { name: 'image', type: 'string', description: 'Find by image template matching' },
      { name: 'optional', type: 'boolean', description: 'Skip if not found' },
      { name: 'desc', type: 'string', description: 'Find by content description/accessibility ID' },
      { name: 'exact', type: 'boolean', description: 'Match text exactly (case-sensitive)' },
      { name: 'retryTapIfNoChange', type: 'boolean', description: 'Retry tap if UI does not change' },
      {
        name: 'scrollable',
        type: 'object',
        description: 'Auto-scroll configuration',
        snippet: 'scrollable:\n    index: ${1:0}\n    itemIndex: ${2:0}'
      },
      // Relative positioning
      {
        name: 'rightOf',
        type: 'string',
        description: 'Find element right of anchor',
        snippet: 'rightOf:\n    text: "${1:text}"'
      },
      {
        name: 'leftOf',
        type: 'string',
        description: 'Find element left of anchor',
        snippet: 'leftOf:\n    text: "${1:text}"'
      },
      {
        name: 'above',
        type: 'string',
        description: 'Find element above anchor',
        snippet: 'above:\n    text: "${1:text}"'
      },
      {
        name: 'below',
        type: 'string',
        description: 'Find element below anchor',
        snippet: 'below:\n    text: "${1:text}"'
      },
      { name: 'label', type: 'string', description: 'Optional label for custom logging' },
      {
        name: 'ocr',
        type: 'object',
        description: 'Find by OCR',
        snippet: 'ocr:\n    text: "${1:text_to_find}"\n    region: "${2|all,top-half,bottom-half,left-half,right-half,center|}"'
      }
    ]
  },
  {
    name: 'tapAt',
    category: 'Interaction',
    description: 'Tap element by type and index',
    hasParams: true,
    snippet: 'tapAt:\n    type: "${1:Button}"\n    index: ${2:0}'
  },
  {
    name: 'inputText',
    aliases: ['write'],
    category: 'Interaction',
    description: 'Input text into focused element',
    hasParams: true,
    snippet: 'write: "$1"',
    params: [
      { name: 'text', type: 'string', description: 'Text to input' },
      { name: 'unicode', type: 'boolean', description: 'Use Unicode mode (supports Vietnamese)' }
    ]
  },
  {
    name: 'type',
    category: 'Interaction',
    description: 'Find element and type text',
    hasParams: true,
    snippet: 'type:\n    text: "$1"\n    selector: "$2"',
    params: [
      { name: 'text', type: 'string', description: 'Text to input' },
      { name: 'selector', type: 'string', description: 'CSS/XPath selector' },
      { name: 'id', type: 'string', description: 'Resource ID' },
      { name: 'xpath', type: 'string', description: 'XPath' },
      { name: 'css', type: 'string', description: 'CSS selector' }
    ]
  },
  {
    name: 'inputAt',
    category: 'Interaction',
    description: 'Input text at element by type and index',
    hasParams: true,
    snippet: 'inputAt:\n    type: "${1:EditText}"\n    index: ${2:0}\n    text: "$3"'
  },
  {
    name: 'eraseText',
    aliases: ['clear'],
    category: 'Interaction',
    description: 'Erase text in focused input',
    hasParams: false
  },
  {
    name: 'hideKeyboard',
    aliases: ['hideKbd'],
    category: 'Interaction',
    description: 'Hide the virtual keyboard',
    hasParams: false
  },
  {
    name: 'press',
    aliases: ['pressKey'],
    category: 'Interaction',
    description: 'Press a physical key (Home, Back, Enter...)',
    hasParams: true,
    snippet: 'press: "${1|Enter,Back,Home|}"',
    params: [
      { name: 'key', type: 'string', description: 'Key name or code' },
      { name: 'times', type: 'number', description: 'Number of times to press' }
    ]
  },
  {
    name: 'back',
    category: 'Interaction',
    description: 'Press Back button',
    hasParams: false
  },
  {
    name: 'home',
    aliases: ['pressHome'],
    category: 'Interaction',
    description: 'Press Home button',
    hasParams: false
  },

  // Scroll & Swipe
  {
    name: 'swipe',
    category: 'Scroll & Swipe',
    description: 'Swipe the screen',
    hasParams: true,
    snippet: 'swipe:\n    direction: "${1|up,down,left,right|}"',
    params: [
      { name: 'direction', type: 'string', description: 'up, down, left, right' },
      { name: 'duration', type: 'number', description: 'Swipe duration in ms' },
      { name: 'distance', type: 'number', description: 'Swipe distance (0-1)' }
    ]
  },
  {
    name: 'swipeUp',
    category: 'Scroll & Swipe',
    description: 'Swipe up',
    hasParams: false
  },
  {
    name: 'swipeDown',
    category: 'Scroll & Swipe',
    description: 'Swipe down',
    hasParams: false
  },
  {
    name: 'swipeLeft',
    category: 'Scroll & Swipe',
    description: 'Swipe left',
    hasParams: false
  },
  {
    name: 'swipeRight',
    category: 'Scroll & Swipe',
    description: 'Swipe right',
    hasParams: false
  },
  {
    name: 'scrollTo',
    aliases: ['scrollUntilVisible'],
    category: 'Scroll & Swipe',
    description: 'Scroll until element is visible',
    hasParams: true,
    snippet: 'scrollTo:\n    ${1|text,id,regex|}: "$2"\n    direction: "${3|down,up,left,right|}"',
    params: [
      { name: 'text', type: 'string', description: 'Find by exact text' },
      { name: 'id', type: 'string', description: 'Find by resource ID' },
      { name: 'css', type: 'string', description: 'Find by CSS selector' },
      { name: 'xpath', type: 'string', description: 'Find by XPath' },
      { name: 'regex', type: 'string', description: 'Find by regex pattern' },
      { name: 'role', type: 'string', description: 'Find by role attribute' },
      { name: 'type', type: 'string', description: 'Element type' },
      { name: 'placeholder', type: 'string', description: 'Find by placeholder text' },
      { name: 'desc', type: 'string', description: 'Find by content description/accessibility ID' },
      { name: 'direction', type: 'string', description: 'Scroll direction: up, down, left, right' },
      { name: 'maxScrolls', type: 'number', description: 'Maximum scroll attempts' },
      { name: 'image', type: 'string', description: 'Find by image template' },
      {
        name: 'from',
        type: 'object',
        description: 'Container element to scroll within',
        snippet: 'from:\n    id: "${1:resource_id}"'
      },
      // Relative positioning
      {
        name: 'rightOf',
        type: 'string',
        description: 'Find element right of anchor',
        snippet: 'rightOf:\n    text: "${1:text}"'
      },
      {
        name: 'leftOf',
        type: 'string',
        description: 'Find element left of anchor',
        snippet: 'leftOf:\n    text: "${1:text}"'
      },
      {
        name: 'above',
        type: 'string',
        description: 'Find element above anchor',
        snippet: 'above:\n    text: "${1:text}"'
      },
      {
        name: 'below',
        type: 'string',
        description: 'Find element below anchor',
        snippet: 'below:\n    text: "${1:text}"'
      },
      { name: 'label', type: 'string', description: 'Optional label for custom logging' },
      {
        name: 'ocr',
        type: 'object',
        description: 'Find by OCR',
        snippet: 'ocr:\n    text: "${1:text_to_find}"\n    region: "${2|all,top-half,bottom-half,left-half,right-half,center|}"'
      }
    ]
  },

  // Assertions
  {
    name: 'see',
    aliases: ['assertVisible'],
    category: 'Assertions',
    description: 'Assert element is visible',
    hasParams: true,
    snippet: 'see: "$1"',
    params: [
      { name: 'text', type: 'string', description: 'Find by exact text' },
      { name: 'id', type: 'string', description: 'Find by resource ID' },
      { name: 'css', type: 'string', description: 'Find by CSS selector' },
      { name: 'xpath', type: 'string', description: 'Find by XPath' },
      { name: 'regex', type: 'string', description: 'Find by regex pattern' },
      { name: 'index', type: 'number', description: 'Element index (0-based)' },
      { name: 'type', type: 'string', description: 'Element type' },
      { name: 'placeholder', type: 'string', description: 'Find by placeholder text' },
      { name: 'desc', type: 'string', description: 'Find by content description/accessibility ID' },
      { name: 'role', type: 'string', description: 'Find by role attribute' },
      { name: 'image', type: 'string', description: 'Find by image template' },
      { name: 'timeout', type: 'number', description: 'Wait timeout in ms' },
      { name: 'soft', type: 'boolean', description: 'Soft assertion (continue on fail)' },
      {
        name: 'scrollable',
        type: 'object',
        description: 'Auto-scroll configuration',
        snippet: 'scrollable:\n    index: ${1:0}\n    itemIndex: ${2:0}'
      },
      {
        name: 'containsChild',
        type: 'object',
        description: 'Assert element contains specific child',
        snippet: 'containsChild:\n    text: "${1:text}"'
      },
      // Relative positioning
      {
        name: 'rightOf',
        type: 'string',
        description: 'Find element right of anchor',
        snippet: 'rightOf:\n    text: "${1:text}"'
      },
      {
        name: 'leftOf',
        type: 'string',
        description: 'Find element left of anchor',
        snippet: 'leftOf:\n    text: "${1:text}"'
      },
      {
        name: 'above',
        type: 'string',
        description: 'Find element above anchor',
        snippet: 'above:\n    text: "${1:text}"'
      },
      {
        name: 'below',
        type: 'string',
        description: 'Find element below anchor',
        snippet: 'below:\n    text: "${1:text}"'
      },
      { name: 'label', type: 'string', description: 'Optional label for custom logging' },
      {
        name: 'ocr',
        type: 'object',
        description: 'Find by OCR',
        snippet: 'ocr:\n    text: "${1:text_to_find}"\n    region: "${2|all,top-half,bottom-half,left-half,right-half,center|}"'
      }
    ]
  },
  {
    name: 'notSee',
    aliases: ['assertNotVisible'],
    category: 'Assertions',
    description: 'Assert element is NOT visible',
    hasParams: true,
    snippet: 'notSee: "$1"',
    params: [
      { name: 'text', type: 'string', description: 'Find by exact text' },
      { name: 'id', type: 'string', description: 'Find by resource ID' },
      { name: 'css', type: 'string', description: 'Find by CSS selector' },
      { name: 'xpath', type: 'string', description: 'Find by XPath' },
      { name: 'regex', type: 'string', description: 'Find by regex pattern' },
      { name: 'index', type: 'number', description: 'Element index (0-based)' },
      { name: 'type', type: 'string', description: 'Element type' },
      { name: 'placeholder', type: 'string', description: 'Find by placeholder text' },
      { name: 'desc', type: 'string', description: 'Find by content description/accessibility ID' },
      { name: 'role', type: 'string', description: 'Find by role attribute' },
      { name: 'timeout', type: 'number', description: 'Wait timeout in ms' },
      {
        name: 'scrollable',
        type: 'object',
        description: 'Auto-scroll configuration',
        snippet: 'scrollable:\n    index: ${1:0}\n    itemIndex: ${2:0}'
      },
      // Relative positioning
      {
        name: 'rightOf',
        type: 'string',
        description: 'Find element right of anchor',
        snippet: 'rightOf:\n    text: "${1:text}"'
      },
      {
        name: 'leftOf',
        type: 'string',
        description: 'Find element left of anchor',
        snippet: 'leftOf:\n    text: "${1:text}"'
      },
      {
        name: 'above',
        type: 'string',
        description: 'Find element above anchor',
        snippet: 'above:\n    text: "${1:text}"'
      },
      {
        name: 'below',
        type: 'string',
        description: 'Find element below anchor',
        snippet: 'below:\n    text: "${1:text}"'
      },
      { name: 'label', type: 'string', description: 'Optional label for custom logging' },
      {
        name: 'ocr',
        type: 'object',
        description: 'Find by OCR',
        snippet: 'ocr:\n    text: "${1:text_to_find}"\n    region: "${2|all,top-half,bottom-half,left-half,right-half,center|}"'
      }
    ]
  },
  {
    name: 'waitUntilVisible',
    aliases: ['waitSee'],
    category: 'Assertions',
    description: 'Wait until element becomes visible',
    hasParams: true,
    snippet: 'waitSee:\n    ${1|text,id,regex|}: "$2"\n    timeout: ${3:5000}',
    params: [
      { name: 'text', type: 'string', description: 'Find by exact text' },
      { name: 'id', type: 'string', description: 'Find by resource ID' },
      { name: 'css', type: 'string', description: 'Find by CSS selector' },
      { name: 'xpath', type: 'string', description: 'Find by XPath' },
      { name: 'regex', type: 'string', description: 'Find by regex pattern' },
      { name: 'index', type: 'number', description: 'Element index (0-based)' },
      { name: 'type', type: 'string', description: 'Element type' },
      { name: 'placeholder', type: 'string', description: 'Find by placeholder text' },
      { name: 'desc', type: 'string', description: 'Find by content description/accessibility ID' },
      { name: 'role', type: 'string', description: 'Find by role attribute' },
      { name: 'timeout', type: 'number', description: 'Wait timeout in ms (default: 5000)' },
      {
        name: 'scrollable',
        type: 'object',
        description: 'Auto-scroll configuration',
        snippet: 'scrollable:\n    index: ${1:0}\n    itemIndex: ${2:0}'
      },
      // Relative positioning
      {
        name: 'rightOf',
        type: 'string',
        description: 'Find element right of anchor',
        snippet: 'rightOf:\n    text: "${1:text}"'
      },
      {
        name: 'leftOf',
        type: 'string',
        description: 'Find element left of anchor',
        snippet: 'leftOf:\n    text: "${1:text}"'
      },
      {
        name: 'above',
        type: 'string',
        description: 'Find element above anchor',
        snippet: 'above:\n    text: "${1:text}"'
      },
      {
        name: 'below',
        type: 'string',
        description: 'Find element below anchor',
        snippet: 'below:\n    text: "${1:text}"'
      },
      { name: 'label', type: 'string', description: 'Optional label for custom logging' },
      {
        name: 'ocr',
        type: 'object',
        description: 'Find by OCR',
        snippet: 'ocr:\n    text: "${1:text_to_find}"\n    region: "${2|all,top-half,bottom-half,left-half,right-half,center|}"'
      }
    ]
  },
  {
    name: 'waitNotSee',
    aliases: ['waitUntilNotVisible'],
    category: 'Assertions',
    description: 'Wait until element disappears',
    hasParams: true,
    snippet: 'waitNotSee:\n    ${1|text,id,regex|}: "$2"\n    timeout: ${3:10000}',
    params: [
      { name: 'text', type: 'string', description: 'Find by exact text' },
      { name: 'id', type: 'string', description: 'Find by resource ID' },
      { name: 'css', type: 'string', description: 'Find by CSS selector' },
      { name: 'xpath', type: 'string', description: 'Find by XPath' },
      { name: 'regex', type: 'string', description: 'Find by regex pattern' },
      { name: 'index', type: 'number', description: 'Element index (0-based)' },
      { name: 'type', type: 'string', description: 'Element type' },
      { name: 'placeholder', type: 'string', description: 'Find by placeholder text' },
      { name: 'desc', type: 'string', description: 'Find by content description/accessibility ID' },
      { name: 'role', type: 'string', description: 'Find by role attribute' },
      { name: 'timeout', type: 'number', description: 'Wait timeout in ms (default: 10000)' },
      {
        name: 'scrollable',
        type: 'object',
        description: 'Auto-scroll configuration',
        snippet: 'scrollable:\n    index: ${1:0}\n    itemIndex: ${2:0}'
      },
      // Relative positioning
      {
        name: 'rightOf',
        type: 'string',
        description: 'Find element right of anchor',
        snippet: 'rightOf:\n    text: "${1:text}"'
      },
      {
        name: 'leftOf',
        type: 'string',
        description: 'Find element left of anchor',
        snippet: 'leftOf:\n    text: "${1:text}"'
      },
      {
        name: 'above',
        type: 'string',
        description: 'Find element above anchor',
        snippet: 'above:\n    text: "${1:text}"'
      },
      {
        name: 'below',
        type: 'string',
        description: 'Find element below anchor',
        snippet: 'below:\n    text: "${1:text}"'
      },
      { name: 'label', type: 'string', description: 'Optional label for custom logging' },
      {
        name: 'ocr',
        type: 'object',
        description: 'Find by OCR',
        snippet: 'ocr:\n    text: "${1:text_to_find}"\n    region: "${2|all,top-half,bottom-half,left-half,right-half,center|}"'
      }
    ]
  },
  {
    name: 'assert',
    aliases: ['assertTrue'],
    category: 'Assertions',
    description: 'Assert a condition is true',
    hasParams: true,
    snippet: 'assert:\n    condition: "${1:\\${count} > 5}"',
    params: [
      { name: 'condition', type: 'string', description: 'Expression to evaluate' },
      { name: 'soft', type: 'boolean', description: 'Soft assertion' }
    ]
  },
  {
    name: 'assertVar',
    category: 'Assertions',
    description: 'Assert variable value',
    hasParams: true,
    snippet: 'assertVar:\n    name: "$1"\n    equals: "$2"'
  },
  {
    name: 'assertColor',
    aliases: ['checkColor'],
    category: 'Assertions',
    description: 'Assert pixel color at point',
    hasParams: true,
    snippet: 'assertColor:\n    point: "${1:50%,50%}"\n    color: "${2:#FF0000}"',
    params: [
      { name: 'point', type: 'string', description: 'Coordinates' },
      { name: 'color', type: 'string', description: 'Hex color code or name' },
      { name: 'tolerance', type: 'number', description: 'Color matching tolerance (0-100)' }
    ]
  },
  {
    name: 'assertScreenshot',
    category: 'Assertions',
    description: 'Compare screen with baseline image',
    hasParams: true,
    snippet: 'assertScreenshot: "$1"'
  },

  // Control Flow
  {
    name: 'wait',
    aliases: ['await'],
    category: 'Control Flow',
    description: 'Wait for specified milliseconds',
    hasParams: true,
    snippet: 'wait: ${1:1000}'
  },
  {
    name: 'waitForAnimationToEnd',
    category: 'Control Flow',
    description: 'Wait for UI to stabilize',
    hasParams: false
  },
  // Variables
  {
    name: 'find',
    category: 'Variables',
    description: 'Define a reusable selector variable',
    hasParams: true,
    snippet: 'find:\n    name: "${1:var_name}"\n    text: "${2:value}"',
    params: [
      { name: 'name', type: 'string', description: 'Variable name', required: true },
      { name: 'text', type: 'string', description: 'Find by exact text' },
      { name: 'id', type: 'string', description: 'Find by resource ID' },
      { name: 'css', type: 'string', description: 'Find by CSS selector' },
      { name: 'xpath', type: 'string', description: 'Find by XPath' },
      { name: 'regex', type: 'string', description: 'Find by regex pattern' },
      { name: 'index', type: 'number', description: 'Element index (0-based)' },
      { name: 'type', type: 'string', description: 'Element type' },
      { name: 'placeholder', type: 'string', description: 'Find by placeholder text' },
      { name: 'desc', type: 'string', description: 'Find by content description/accessibility ID' },
      { name: 'role', type: 'string', description: 'Find by role attribute' },
      { name: 'image', type: 'string', description: 'Find by image template' },
      {
        name: 'scrollable',
        type: 'object',
        description: 'Auto-scroll configuration',
        snippet: 'scrollable:\n    index: ${1:0}\n    itemIndex: ${2:0}'
      },
      // Relative positioning
      {
        name: 'rightOf',
        type: 'string',
        description: 'Find element right of anchor',
        snippet: 'rightOf:\n    text: "${1:text}"'
      },
      {
        name: 'leftOf',
        type: 'string',
        description: 'Find element left of anchor',
        snippet: 'leftOf:\n    text: "${1:text}"'
      },
      {
        name: 'above',
        type: 'string',
        description: 'Find element above anchor',
        snippet: 'above:\n    text: "${1:text}"'
      },
      {
        name: 'below',
        type: 'string',
        description: 'Find element below anchor',
        snippet: 'below:\n    text: "${1:text}"'
      },
      {
        name: 'ocr',
        type: 'object',
        description: 'Find by OCR',
        snippet: 'ocr:\n    text: "${1:text_to_find}"\n    region: "${2|all,top-half,bottom-half,left-half,right-half,center|}"'
      }
    ]
  },
  {
    name: 'setVar',
    category: 'Control Flow',
    description: 'Set a variable',
    hasParams: true,
    snippet: 'setVar:\n    name: "$1"\n    value: "$2"'
  },
  {
    name: 'runFlow',
    category: 'Control Flow',
    description: 'Run a sub-flow',
    hasParams: true,
    snippet: 'runFlow: "$1"'
  },
  {
    name: 'repeat',
    category: 'Control Flow',
    description: 'Repeat commands',
    hasParams: true,
    snippet: 'repeat:\n    times: ${1:5}\n    commands:\n        - $0'
  },
  {
    name: 'retry',
    category: 'Control Flow',
    description: 'Retry commands on failure',
    hasParams: true,
    snippet: 'retry:\n    times: ${1:3}\n    commands:\n        - $0'
  },
  {
    name: 'conditional',
    category: 'Control Flow',
    description: 'If-else condition',
    hasParams: true,
    snippet: 'conditional:\n    if:\n        - see: "$1"\n    then:\n        - $0'
  },
  {
    name: 'runScript',
    category: 'Control Flow',
    description: 'Run shell script',
    hasParams: true,
    snippet: 'runScript: "$1"',
    params: [
      { name: 'command', type: 'string', description: 'Script command/path' },
      { name: 'args', type: 'object', description: 'Arguments list' },
      { name: 'saveOutput', type: 'string', description: 'Variable to save stdout' },
      { name: 'timeoutMs', type: 'number', description: 'Timeout in ms' },
      { name: 'failOnError', type: 'boolean', description: 'Fail test if script exits with error' }
    ]
  },
  {
    name: 'evalScript',
    category: 'Control Flow',
    description: 'Evaluate JavaScript expression',
    hasParams: true,
    snippet: 'evalScript: "$1"'
  },
  {
    name: 'httpRequest',
    category: 'Control Flow',
    description: 'Send HTTP request',
    hasParams: true,
    snippet: 'httpRequest:\n    url: "$1"\n    method: "${2|GET,POST,PUT,DELETE|}"',
    params: [
      { name: 'url', type: 'string', description: 'Request URL' },
      { name: 'method', type: 'string', description: 'HTTP Method' },
      { name: 'headers', type: 'object', description: 'HTTP Headers' },
      { name: 'body', type: 'object', description: 'Request Body' },
      { name: 'saveResponse', type: 'object', description: 'Map response JSON paths to variables' }
    ]
  },

  // Media
  {
    name: 'takeScreenshot',
    aliases: ['screenshot'],
    category: 'Media',
    description: 'Take a screenshot',
    hasParams: true,
    snippet: 'takeScreenshot: "$1.png"'
  },
  {
    name: 'startRecording',
    category: 'Media',
    description: 'Start video recording',
    hasParams: true,
    snippet: 'startRecording: "$1"'
  },
  {
    name: 'stopRecording',
    category: 'Media',
    description: 'Stop video recording',
    hasParams: false
  },
  {
    name: 'startGifCapture',
    category: 'Media',
    description: 'Start capturing frames for GIF',
    hasParams: true,
    snippet: 'startGifCapture:\n    interval: ${1:500}\n    maxFrames: ${2:100}'
  },
  {
    name: 'stopGifCapture',
    category: 'Media',
    description: 'Stop GIF capture and save',
    hasParams: true,
    snippet: 'stopGifCapture: "$1.gif"'
  },

  // Mock Location
  {
    name: 'mockLocation',
    aliases: ['gps'],
    category: 'Mock Location',
    description: 'Simulate GPS location from file',
    hasParams: true,
    snippet: 'gps:\n    file: "$1.gpx"\n    speed: ${2:40}',
    params: [
      { name: 'file', type: 'string', description: 'Path to GPX/KML file' },
      { name: 'speed', type: 'number', description: 'Speed in km/h' },
      { name: 'altitude', type: 'number', description: 'Fixed altitude in meters (overrides file)' },
      { name: 'loop', type: 'boolean', description: 'Loop playback' },
      { name: 'speedMode', type: 'string', description: 'linear or noise' },
      { name: 'speedNoise', type: 'number', description: 'Noise amount for speed' },
      { name: 'startIndex', type: 'number', description: 'Start index' },
      { name: 'intervalMs', type: 'number', description: 'Update interval' }
    ]
  },
  {
    name: 'stopMockLocation',
    aliases: ['stopGps'],
    category: 'Mock Location',
    description: 'Stop GPS simulation',
    hasParams: false
  },
  {
    name: 'mockLocationControl',
    category: 'Mock Location',
    description: 'Control GPS playback (speed, pause)',
    hasParams: true,
    snippet: 'mockLocationControl:\n    speed: ${1:60}'
  },

  // System
  {
    name: 'openNotifications',
    category: 'System',
    description: 'Open notification panel',
    hasParams: false
  },
  {
    name: 'openQuickSettings',
    category: 'System',
    description: 'Open quick settings',
    hasParams: false
  },
  {
    name: 'setVolume',
    category: 'System',
    description: 'Set volume level',
    hasParams: true,
    snippet: 'setVolume: ${1:50}'
  },
  {
    name: 'lockDevice',
    category: 'System',
    description: 'Lock device screen',
    hasParams: false
  },
  {
    name: 'unlockDevice',
    category: 'System',
    description: 'Unlock device screen',
    hasParams: false
  },
  {
    name: 'setNetwork',
    category: 'System',
    description: 'Toggle WiFi/Data',
    hasParams: true,
    snippet: 'setNetwork:\n    wifi: ${1|true,false|}'
  },
  {
    name: 'airplaneMode',
    category: 'System',
    description: 'Toggle airplane mode',
    hasParams: false
  },
  {
    name: 'setOrientation',
    aliases: ['rotate'],
    category: 'System',
    description: 'Set screen orientation',
    hasParams: true,
    snippet: 'rotate: "${1|portrait,landscape|}"'
  },
  {
    name: 'setLocale',
    category: 'System',
    description: 'Set device locale (Android only)',
    hasParams: true,
    snippet: 'setLocale: "${1:en_US}"',
    platforms: ['android']
  },
  {
    name: 'sendLarkMessage',
    category: 'System',
    description: 'Send a notification to Lark/Feishu',
    hasParams: true,
    snippet: `sendLarkMessage:
    webhook: "\${1:https://...}"
    secret: "\${2:optional_secret}"
    title: "\${3:Test Report}"
    content: "\${4:Tests completed}"
    status: "\${5|success,failure,info,warning|}"`,
    platforms: ['android', 'ios', 'web']
  },

  // Clipboard
  {
    name: 'setClipboard',
    category: 'Clipboard',
    description: 'Set clipboard content',
    hasParams: true,
    snippet: 'setClipboard: "$1"'
  },
  {
    name: 'getClipboard',
    category: 'Clipboard',
    description: 'Get clipboard to variable',
    hasParams: true,
    snippet: 'getClipboard:\n    name: "$1"'
  },
  {
    name: 'assertClipboard',
    category: 'Clipboard',
    description: 'Assert clipboard content',
    hasParams: true,
    snippet: 'assertClipboard: "$1"'
  },
  {
    name: 'copyTextFrom',
    category: 'Clipboard',
    description: 'Copy text from element',
    hasParams: true,
    snippet: 'copyTextFrom:\n    id: "$1"'
  },
  {
    name: 'pasteText',
    category: 'Clipboard',
    description: 'Paste from clipboard',
    hasParams: false
  },

  // Random Input
  {
    name: 'inputRandomEmail',
    category: 'Random Input',
    description: 'Input random email',
    hasParams: false
  },
  {
    name: 'inputRandomNumber',
    aliases: ['inputRandomPhoneNumber'],
    category: 'Random Input',
    description: 'Input random number',
    hasParams: true,
    snippet: 'inputRandomNumber:\n    length: ${1:6}'
  },
  {
    name: 'inputRandomPersonName',
    category: 'Random Input',
    description: 'Input random person name',
    hasParams: false
  },
  {
    name: 'inputRandomText',
    category: 'Random Input',
    description: 'Input random text',
    hasParams: true,
    snippet: 'inputRandomText:\n    length: ${1:10}'
  },
  {
    name: 'generate',
    category: 'Random Input',
    description: 'Generate fake data to variable',
    hasParams: true,
    snippet: 'generate:\n    name: "$1"\n    type: "${2|email,name,phone,uuid|}"'
  },

  // File Transfer
  {
    name: 'pushFile',
    category: 'File Transfer',
    description: 'Push file to device',
    hasParams: true,
    snippet: 'pushFile:\n    src: "$1"\n    dest: "$2"'
  },
  {
    name: 'pullFile',
    category: 'File Transfer',
    description: 'Pull file from device',
    hasParams: true,
    snippet: 'pullFile:\n    src: "$1"\n    dest: "$2"'
  },

  // Deep Link
  {
    name: 'openLink',
    aliases: ['deepLink'],
    category: 'Navigation',
    description: 'Open deep link URL',
    hasParams: true,
    snippet: 'openLink: "$1"'
  },
  {
    name: 'navigate',
    category: 'Navigation',
    description: 'Navigate to URL (Web)',
    hasParams: true,
    snippet: 'navigate: "$1"'
  },

  // Performance & Profiling
  {
    name: 'startProfiling',
    category: 'Performance',
    description: 'Start collecting performance metrics (CPU, Memory, FPS)',
    hasParams: true,
    snippet: 'startProfiling:\n    interval: ${1:1000}\n    output: "${2:profile.json}"',
    params: [
      { name: 'interval', type: 'number', description: 'Sampling interval in ms' },
      { name: 'output', type: 'string', description: 'Output file path' },
      { name: 'metrics', type: 'string', description: 'Metrics to collect: cpu,memory,fps' }
    ]
  },
  {
    name: 'stopProfiling',
    category: 'Performance',
    description: 'Stop profiling and save results',
    hasParams: true,
    snippet: 'stopProfiling:\n    output: "${1:profile.json}"',
    params: [
      { name: 'output', type: 'string', description: 'Output file path' }
    ]
  },
  {
    name: 'assertPerformance',
    category: 'Performance',
    description: 'Assert performance metrics are within thresholds',
    hasParams: true,
    snippet: 'assertPerformance:\n    ${1|cpu,memory,fps|}: ${2:50}',
    params: [
      { name: 'cpu', type: 'number', description: 'Max CPU usage %' },
      { name: 'memory', type: 'number', description: 'Max memory in MB' },
      { name: 'fps', type: 'number', description: 'Min FPS' },
      { name: 'metric', type: 'string', description: 'Metric name (cpu, memory, fps, jank)' },
      { name: 'limit', type: 'string', description: 'Threshold limit (e.g. 250MB)' }
    ]
  },
  {
    name: 'setCpuThrottling',
    category: 'Performance',
    description: 'Set CPU throttling rate (Web)',
    hasParams: true,
    snippet: 'setCpuThrottling: ${1:4}'
  },
  {
    name: 'setNetworkConditions',
    category: 'Performance',
    description: 'Set network conditions (Slow 3G, Fast 3G, Offline)',
    hasParams: true,
    snippet: 'setNetworkConditions: "${1|Slow 3G,Fast 3G,Regular 4G,Offline|}"'
  },

  // Database
  {
    name: 'dbQuery',
    category: 'Database',
    description: 'Execute database query',
    hasParams: true,
    snippet: 'dbQuery:\n    query: "${1:SELECT * FROM users}"\n    connection: "${2:sqlite:./test.db}"',
    params: [
      { name: 'query', type: 'string', description: 'SQL query to execute' },
      { name: 'connection', type: 'string', description: 'Connection string' },
      { name: 'params', type: 'object', description: 'Binding parameters' },
      { name: 'save', type: 'object', description: 'Map columns to variables' }
    ]
  },

  // GIF Frame Control
  {
    name: 'captureFrame',
    aliases: ['captureGifFrame'],
    category: 'Media',
    description: 'Capture single frame for GIF',
    hasParams: true,
    snippet: 'captureFrame:\n    name: "${1:frame}"',
    params: [
      { name: 'name', type: 'string', description: 'Frame name/prefix' }
    ]
  },
  {
    name: 'buildGif',
    aliases: ['createGif'],
    category: 'Media',
    description: 'Build GIF from captured frames',
    hasParams: true,
    snippet: 'buildGif:\n    output: "${1:output.gif}"\n    delay: ${2:500}',
    params: [
      { name: 'output', type: 'string', description: 'Output GIF file path' },
      { name: 'delay', type: 'number', description: 'Delay between frames in ms' },
      { name: 'loop', type: 'boolean', description: 'Loop GIF (default: true)' }
    ]
  },

  // Mock Location Sync
  {
    name: 'waitForLocation',
    category: 'Mock Location',
    description: 'Wait until device reaches a GPS location',
    hasParams: true,
    snippet: 'waitForLocation:\n    lat: ${1:10.762}\n    lon: ${2:106.660}\n    tolerance: ${3:50}',
    params: [
      { name: 'lat', type: 'number', description: 'Target latitude' },
      { name: 'lon', type: 'number', description: 'Target longitude' },
      { name: 'tolerance', type: 'number', description: 'Tolerance in meters' },
      { name: 'timeout', type: 'number', description: 'Timeout in ms' }
    ]
  },
  {
    name: 'waitForMockCompletion',
    category: 'Mock Location',
    description: 'Wait for mock GPS playback to complete',
    hasParams: true,
    snippet: 'waitForMockCompletion:\n    timeout: ${1:60000}',
    params: [
      { name: 'name', type: 'string', description: 'Mock instance name' },
      { name: 'timeout', type: 'number', description: 'Timeout in ms' }
    ]
  },

  // Extended Wait
  {
    name: 'extendedWaitUntil',
    category: 'Control Flow',
    description: 'Wait with multiple conditions',
    hasParams: true,
    snippet: 'extendedWaitUntil:\n    conditions:\n        - see: "$1"\n    timeout: ${2:10000}',
    params: [
      { name: 'conditions', type: 'object', description: 'List of conditions' },
      { name: 'timeout', type: 'number', description: 'Timeout in ms' },
      { name: 'interval', type: 'number', description: 'Check interval in ms' }
    ]
  },

  // Report
  {
    name: 'exportReport',
    category: 'Report',
    description: 'Export test report',
    hasParams: true,
    snippet: 'exportReport:\n    format: "${1|html,json|}"\n    output: "${2:report}"',
    params: [
      { name: 'format', type: 'string', description: 'Report format: html, json' },
      { name: 'output', type: 'string', description: 'Output file path' }
    ]
  },

  // Hardware Camera Verification
  {
    name: 'assertDeviceState',
    aliases: ['checkDevice'],
    category: 'Hardware',
    description: 'Assert a physical device LED state from the configured camera',
    hasParams: true,
    snippet: 'assertDeviceState:\n    button: "${1:\\${TARGET_DEVICE}.button_1}"\n    expect: "${2:\\${STATE_PRIMARY}}"',
    params: [
      { name: 'button', type: 'string', description: 'Region id from the camera profile, preferably device-qualified such as ${TARGET_DEVICE}.button_1' },
      { name: 'led', type: 'string', description: 'Alias for button' },
      { name: 'region', type: 'string', description: 'Alias for button' },
      { name: 'expect', type: 'string', description: 'Expected state/color' },
      { name: 'camera', type: 'string', description: 'Named camera when using cameras:' }
    ]
  },
  {
    name: 'waitDeviceState',
    category: 'Hardware',
    description: 'Wait until a physical device LED reaches the expected state',
    hasParams: true,
    snippet: 'waitDeviceState:\n    button: "${1:\\${TARGET_DEVICE}.button_1}"\n    expect: "${2:\\${STATE_REST}}"',
    params: [
      { name: 'button', type: 'string', description: 'Region id from the camera profile, preferably device-qualified such as ${TARGET_DEVICE}.button_1' },
      { name: 'led', type: 'string', description: 'Alias for button' },
      { name: 'region', type: 'string', description: 'Alias for button' },
      { name: 'expect', type: 'string', description: 'Expected state/color' },
      { name: 'timeoutMs', type: 'number', description: 'Optional timeout override in ms' },
      { name: 'stableFrames', type: 'number', description: 'Consecutive matching frames required' },
      { name: 'camera', type: 'string', description: 'Named camera when using cameras:' }
    ]
  },
  {
    name: 'assertDeviceTransition',
    aliases: ['assertDeviceChange'],
    category: 'Hardware',
    description: 'Assert a physical LED starts at one state and transitions to another',
    hasParams: true,
    snippet: 'assertDeviceTransition:\n    button: "${1:\\${TARGET_DEVICE}.button_1}"\n    from: "${2:OFF}"\n    to: "${3:\\${STATE_PRIMARY}}"',
    params: [
      { name: 'button', type: 'string', description: 'Region id from the camera profile, preferably device-qualified such as ${TARGET_DEVICE}.button_1' },
      { name: 'led', type: 'string', description: 'Alias for button' },
      { name: 'region', type: 'string', description: 'Alias for button' },
      { name: 'from', type: 'string', description: 'Required starting state' },
      { name: 'to', type: 'string', description: 'Target state' },
      { name: 'timeoutMs', type: 'number', description: 'Optional timeout override in ms' },
      { name: 'stableFrames', type: 'number', description: 'Consecutive matching frames required' },
      { name: 'camera', type: 'string', description: 'Named camera when using cameras:' }
    ]
  },
  {
    name: 'waitLedPattern',
    aliases: ['assertDevicePattern'],
    category: 'Hardware',
    description: 'Wait for a blink/reset pattern from camera frame timestamps',
    hasParams: true,
    snippet: 'waitLedPattern:\n    button: "${1:\\${TARGET_DEVICE}.status}"\n    expect: "${2:PINK}"\n    count: ${3:3}\n    withinMs: ${4:800}',
    params: [
      { name: 'button', type: 'string', description: 'Region id from the camera profile, preferably device-qualified such as ${TARGET_DEVICE}.status' },
      { name: 'led', type: 'string', description: 'Alias for button' },
      { name: 'region', type: 'string', description: 'Alias for button' },
      { name: 'expect', type: 'string', description: 'Pulse state/color' },
      { name: 'count', type: 'number', description: 'Required pulse count' },
      { name: 'withinMs', type: 'number', description: 'Max pattern duration in ms' },
      { name: 'pulseMinMs', type: 'number', description: 'Optional minimum pulse duration' },
      { name: 'pulseMaxMs', type: 'number', description: 'Optional maximum pulse duration' },
      { name: 'sampleMs', type: 'number', description: 'Camera sampling interval' },
      { name: 'camera', type: 'string', description: 'Named camera when using cameras:' }
    ]
  },
  {
    name: 'getDeviceState',
    category: 'Hardware',
    description: 'Read all camera profile regions into a JSON variable and artifact',
    hasParams: true,
    snippet: 'getDeviceState:\n    saveAs: "${1:deviceState}"',
    params: [
      { name: 'saveAs', type: 'string', description: 'Variable name for the JSON device state' },
      { name: 'camera', type: 'string', description: 'Named camera when using cameras:' }
    ]
  },

  // Audio Test Commands
  {
    name: 'playMedia',
    aliases: ['playMedia'],
    category: 'Audio',
    description: 'Play audio/video file on device',
    hasParams: true,
    snippet: 'playMedia:\n    file: "$1"',
    params: [
      { name: 'file', type: 'string', description: 'Path to audio file (relative or absolute)' },
      { name: 'loopPlayback', type: 'boolean', description: 'Loop playback continuously' }
    ],
    platforms: ['android']
  },
  {
    name: 'stopMedia',
    category: 'Audio',
    description: 'Stop media playback',
    hasParams: false,
    platforms: ['android']
  },
  {
    name: 'startAudioCapture',
    category: 'Audio',
    description: 'Start capturing device audio for analysis',
    hasParams: true,
    snippet: 'startAudioCapture:\n    duration: ${1:30000}',
    params: [
      { name: 'duration', type: 'number', description: 'Capture duration in ms (default: 30000)' },
      { name: 'port', type: 'number', description: 'Audio server port (default: 8890)' }
    ],
    platforms: ['android']
  },
  {
    name: 'stopAudioCapture',
    category: 'Audio',
    description: 'Stop audio capture',
    hasParams: false,
    platforms: ['android']
  },
  {
    name: 'verifyAudioDucking',
    category: 'Audio',
    description: 'Verify audio ducking occurred during capture',
    hasParams: true,
    snippet: 'verifyAudioDucking:\n    minDuckingCount: ${1:1}',
    params: [
      { name: 'minDuckingCount', type: 'number', description: 'Minimum ducking events required (default: 1)' },
      { name: 'volumeDropThreshold', type: 'number', description: 'Minimum volume drop percentage (default: 30)' }
    ],
    platforms: ['android']
  },

  // Hardware Automation (Native RS485/Serial)
  {
    name: 'connectJig',
    category: 'Hardware Automation',
    description: 'Connect to hardware Jig controller via serial port',
    hasParams: true,
    snippet: 'connectJig: "${1:COM5}"',
    params: [
      { name: 'port', type: 'string', description: 'Serial port name (e.g. COM5 or /dev/ttyUSB0)' },
      { name: 'baudrate', type: 'number', description: 'Serial baud rate (default: 115200)' },
      { name: 'autoPowerOff', type: 'boolean', description: 'Auto turn off power when test finishes' },
      { name: 'timeoutMs', type: 'number', description: 'Connection timeout in ms' }
    ]
  },
  {
    name: 'disconnectJig',
    category: 'Hardware Automation',
    description: 'Disconnect hardware Jig controller',
    hasParams: false
  },
  {
    name: 'turnOn',
    aliases: ['relayOn'],
    category: 'Hardware Automation',
    description: 'Turn ON relay power channel',
    hasParams: true,
    snippet: 'turnOn: ${1:1}',
    params: [
      { name: 'channel', type: 'number', description: 'Relay channel number (1..4)' }
    ]
  },
  {
    name: 'turnOff',
    aliases: ['relayOff'],
    category: 'Hardware Automation',
    description: 'Turn OFF relay power channel',
    hasParams: true,
    snippet: 'turnOff: ${1:1}',
    params: [
      { name: 'channel', type: 'number', description: 'Relay channel number (1..4)' }
    ]
  },
  {
    name: 'turnOffAll',
    aliases: ['relayAllOff'],
    category: 'Hardware Automation',
    description: 'Turn OFF all relay power channels',
    hasParams: false
  },
  {
    name: 'powerCycle',
    category: 'Hardware Automation',
    description: 'Hard Power Reboot (Turn off -> wait -> Turn on)',
    hasParams: true,
    snippet: 'powerCycle: ${1:1}',
    params: [
      { name: 'channel', type: 'number', description: 'Relay channel number (1..4)' },
      { name: 'offMs', type: 'number', description: 'Off duration in milliseconds (default: 1000)' }
    ]
  },
  {
    name: 'clickButton',
    aliases: ['click'],
    category: 'Hardware Automation',
    description: 'Click physical button via servo motor',
    hasParams: true,
    snippet: 'clickButton: ${1:1}',
    params: [
      { name: 'channel', type: 'number', description: 'Servo channel number (1..8)' },
      { name: 'holdMs', type: 'number', description: 'Button hold duration in milliseconds (default: 300)' }
    ]
  },
  {
    name: 'repeatClick',
    category: 'Hardware Automation',
    description: 'Click physical button N times (Multi-tap)',
    hasParams: true,
    snippet: 'repeatClick:\n    channel: ${1:1}\n    count: ${2:3}',
    params: [
      { name: 'channel', type: 'number', description: 'Servo channel number (1..8)' },
      { name: 'count', type: 'number', description: 'Number of repetitions (e.g. 3)' },
      { name: 'pressMs', type: 'number', description: 'Press duration per click in ms (default: 200)' },
      { name: 'releaseMs', type: 'number', description: 'Release duration between clicks in ms (default: 200)' }
    ]
  },
  {
    name: 'pressButton',
    aliases: ['press', 'holdButton', 'hold'],
    category: 'Hardware Automation',
    description: 'Press and hold physical button (Pairing/Reset)',
    hasParams: true,
    snippet: 'pressButton: ${1:1}',
    params: [
      { name: 'channel', type: 'number', description: 'Servo channel number (1..8)' }
    ]
  },
  {
    name: 'holdButton',
    aliases: ['press', 'pressButton', 'hold'],
    category: 'Hardware Automation',
    description: 'Press and hold physical button (Pairing/Reset)',
    hasParams: true,
    snippet: 'holdButton: ${1:1}',
    params: [
      { name: 'channel', type: 'number', description: 'Servo channel number (1..8)' }
    ]
  },
  {
    name: 'releaseButton',
    aliases: ['release'],
    category: 'Hardware Automation',
    description: 'Release held physical button',
    hasParams: true,
    snippet: 'releaseButton: ${1:1}',
    params: [
      { name: 'channel', type: 'number', description: 'Servo channel number (1..8)' }
    ]
  },
  {
    name: 'readServo',
    aliases: ['getServoState', 'servoState'],
    category: 'Hardware Automation',
    description: 'Read servo state and angle for channel',
    hasParams: true,
    snippet: 'readServo: ${1:1}',
    params: [
      { name: 'channel', type: 'number', description: 'Servo channel number (1..8)' }
    ]
  },
  {
    name: 'readRelay',
    aliases: ['getRelayState', 'relayState'],
    category: 'Hardware Automation',
    description: 'Read relay power state (ON/OFF) for channel',
    hasParams: true,
    snippet: 'readRelay: ${1:1}',
    params: [
      { name: 'channel', type: 'number', description: 'Relay channel number (1..4)' }
    ]
  },
  {
    name: 'readColor',
    aliases: ['readColorSensor', 'colorState'],
    category: 'Hardware Automation',
    description: 'Read color sensor RGBC sample and color confidence for channel',
    hasParams: true,
    snippet: 'readColor: ${1:1}',
    params: [
      { name: 'channel', type: 'number', description: 'Color sensor channel number (1..8)' }
    ]
  },
  {
    name: 'readSensorLight',
    aliases: ['getSensorLightState', 'lightState'],
    category: 'Hardware Automation',
    description: 'Read color sensor LED light state (PB15 ON/OFF)',
    hasParams: false
  },
  {
    name: 'releaseAllButtons',
    aliases: ['releaseAll'],
    category: 'Hardware Automation',
    description: 'Release all servo buttons to idle position',
    hasParams: false
  },
  {
    name: 'startRepeatClick',
    category: 'Hardware Automation',
    description: 'Start continuous button click repeat on STM32',
    hasParams: true,
    snippet: 'startRepeatClick:\n    channel: ${1:1}\n    periodMs: ${2:1500}',
    params: [
      { name: 'channel', type: 'number', description: 'Servo channel number (1..8)' },
      { name: 'periodMs', type: 'number', description: 'Repetition period in milliseconds (e.g. 1500)' }
    ]
  },
  {
    name: 'stopRepeatClick',
    category: 'Hardware Automation',
    description: 'Stop continuous button click repeat on STM32',
    hasParams: true,
    snippet: 'stopRepeatClick: ${1:1}',
    params: [
      { name: 'channel', type: 'number', description: 'Servo channel number (1..8)' }
    ]
  },
  {
    name: 'configureServo',
    aliases: ['setServoConfig'],
    category: 'Hardware Automation',
    description: 'Configure servo channel angles and motion durations',
    hasParams: true,
    snippet: 'configureServo:\n    channel: ${1:1}\n    pressAngle: ${2:75}\n    releaseAngle: ${3:15}',
    params: [
      { name: 'channel', type: 'number', description: 'Servo channel number (1..8)' },
      { name: 'pressAngle', type: 'number', description: 'Angle when button is pressed (default: 72/75)' },
      { name: 'releaseAngle', type: 'number', description: 'Angle when button is released (default: 15)' },
      { name: 'pressDurationMs', type: 'number', description: 'Motion time to press in ms (default: 400)' },
      { name: 'releaseDurationMs', type: 'number', description: 'Motion time to release in ms (default: 150)' },
      { name: 'holdDurationMs', type: 'number', description: 'Hold time for click action in ms (default: 300)' }
    ]
  },
  {
    name: 'seeLedColor',
    category: 'Hardware Automation',
    description: 'Assert/Wait for LED color reading from TCS sensor',
    hasParams: true,
    snippet: 'seeLedColor: "${1:GREEN}"',
    params: [
      { name: 'channel', type: 'number', description: 'Color sensor channel number (1..8)' },
      { name: 'expected', type: 'object', description: 'Expected color string or list (e.g. ["RED", "GREEN"])' },
      { name: 'timeoutMs', type: 'number', description: 'Timeout in milliseconds (default: 5000)' }
    ]
  },
  {
    name: 'seeLedBlink',
    category: 'Hardware Automation',
    description: 'Assert/Wait for LED blink pattern detection',
    hasParams: true,
    snippet: 'seeLedBlink: ${1:1}',
    params: [
      { name: 'channel', type: 'number', description: 'Color sensor channel number (1..8)' },
      { name: 'timeoutMs', type: 'number', description: 'Timeout in milliseconds (default: 5000)' }
    ]
  },
  {
    name: 'seeLedOff',
    category: 'Hardware Automation',
    description: 'Assert/Wait for LED to turn completely OFF',
    hasParams: true,
    snippet: 'seeLedOff: ${1:1}',
    params: [
      { name: 'channel', type: 'number', description: 'Color sensor channel number (1..8)' },
      { name: 'timeoutMs', type: 'number', description: 'Timeout in milliseconds (default: 5000)' }
    ]
  },
  {
    name: 'setSensorLight',
    aliases: ['toggleLight', 'sensorLight'],
    category: 'Hardware Automation',
    description: 'Turn color sensor illumination light ON or OFF',
    hasParams: true,
    snippet: 'setSensorLight: "${1|on,off|}"',
    params: [
      { name: 'enabled', type: 'boolean', description: 'Light enabled state (true/false or on/off)' }
    ]
  },
  {
    name: 'setBrightnessThresholds',
    category: 'Hardware Automation',
    description: 'Configure brightness thresholds and blink detection window',
    hasParams: true,
    snippet: 'setBrightnessThresholds:\n    channel: ${1:1}\n    offBelowPercent: ${2:30}\n    onAbovePercent: ${3:70}',
    params: [
      { name: 'channel', type: 'number', description: 'Color sensor channel number (1..8)' },
      { name: 'offBelowPercent', type: 'number', description: 'OFF threshold percentage (0..99)' },
      { name: 'onAbovePercent', type: 'number', description: 'ON threshold percentage (1..100)' },
      { name: 'minPulseMs', type: 'number', description: 'Minimum pulse duration in ms (default: 50)' },
      { name: 'maxPulseMs', type: 'number', description: 'Maximum pulse duration in ms (default: 1000)' },
      { name: 'sequenceEndGapMs', type: 'number', description: 'End gap duration in ms (default: 500)' }
    ]
  },
  {
    name: 'waitForBrightness',
    category: 'Hardware Automation',
    description: 'Wait for brightness percentage in range',
    hasParams: true,
    snippet: 'waitForBrightness:\n    channel: ${1:1}\n    minPercent: ${2:70}',
    params: [
      { name: 'channel', type: 'number', description: 'Color sensor channel number (1..8)' },
      { name: 'minPercent', type: 'number', description: 'Minimum brightness percentage' },
      { name: 'maxPercent', type: 'number', description: 'Maximum brightness percentage' },
      { name: 'timeoutMs', type: 'number', description: 'Timeout in milliseconds (default: 5000)' }
    ]
  },
  {
    name: 'waitForCct',
    category: 'Hardware Automation',
    description: 'Wait for CCT color temperature in Kelvin',
    hasParams: true,
    snippet: 'waitForCct:\n    channel: ${1:1}\n    minKelvin: ${2:2700}\n    maxKelvin: ${3:6500}',
    params: [
      { name: 'channel', type: 'number', description: 'Color sensor channel number (1..8)' },
      { name: 'minKelvin', type: 'number', description: 'Minimum color temperature in Kelvin' },
      { name: 'maxKelvin', type: 'number', description: 'Maximum color temperature in Kelvin' },
      { name: 'timeoutMs', type: 'number', description: 'Timeout in milliseconds (default: 5000)' }
    ]
  },
  {
    name: 'calibrateColor',
    category: 'Hardware Automation',
    description: 'Calibrate reference color (RED, GREEN, BLUE, YELLOW, CYAN, MAGENTA, PINK, WHITE)',
    hasParams: true,
    snippet: 'calibrateColor:\n    channel: ${1:1}\n    color: "${2|RED,GREEN,BLUE,YELLOW,CYAN,MAGENTA,PINK,WHITE|}"',
    params: [
      { name: 'channel', type: 'number', description: 'Color sensor channel number (1..8)' },
      { name: 'color', type: 'string', description: 'Target reference color name' }
    ]
  },
  {
    name: 'calibrateBrightness',
    category: 'Hardware Automation',
    description: 'Calibrate reference brightness (dark or on)',
    hasParams: true,
    snippet: 'calibrateBrightness:\n    channel: ${1:1}\n    mode: "${2|dark,on|}"',
    params: [
      { name: 'channel', type: 'number', description: 'Color sensor channel number (1..8)' },
      { name: 'mode', type: 'string', description: 'Calibration mode: dark (LED off) or on (LED on)' }
    ]
  },
  {
    name: 'addCctPoint',
    category: 'Hardware Automation',
    description: 'Add CCT calibration point in Kelvin',
    hasParams: true,
    snippet: 'addCctPoint:\n    channel: ${1:1}\n    knownKelvin: ${2:4000}',
    params: [
      { name: 'channel', type: 'number', description: 'Color sensor channel number (1..8)' },
      { name: 'knownKelvin', type: 'number', description: 'Known CCT Kelvin value (e.g. 2700, 4000, 6500)' }
    ]
  },
  {
    name: 'saveCalibration',
    category: 'Hardware Automation',
    description: 'Save calibration data to MCU Flash memory',
    hasParams: false
  },
  {
    name: 'loadCalibration',
    category: 'Hardware Automation',
    description: 'Load calibration data from MCU Flash memory',
    hasParams: false
  },
  {
    name: 'resetCalibration',
    category: 'Hardware Automation',
    description: 'Reset calibration data to factory defaults',
    hasParams: false
  },
  {
    name: 'eraseCalibration',
    category: 'Hardware Automation',
    description: 'Erase MCU Flash calibration data',
    hasParams: false
  },
  {
    name: 'enterSafeState',
    category: 'Hardware Automation',
    description: 'Trigger hardware safety shutdown (relays off, servos released, sensor light off)',
    hasParams: false
  },
  {
    name: 'systemDiagnostics',
    category: 'Hardware Automation',
    description: 'Query hardware diagnostics from MCU',
    hasParams: false
  }
];
