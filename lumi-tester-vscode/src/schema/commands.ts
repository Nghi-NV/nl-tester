// Lumi Tester Command Definitions
// Auto-generated from lumi-tester/docs/commands.md

export interface CommandParam {
  name: string;
  type: 'string' | 'number' | 'boolean' | 'object';
  description: string;
  required?: boolean;
}

export interface LumiCommand {
  name: string;
  aliases?: string[];
  category: string;
  description: string;
  hasParams: boolean;
  snippet?: string;
  params?: CommandParam[];
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
      { name: 'permissions', type: 'object', description: 'Permissions to set' }
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
      { name: 'optional', type: 'boolean', description: 'Skip if not found' }
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
      { name: 'image', type: 'string', description: 'Find by image template matching' }
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
      { name: 'image', type: 'string', description: 'Find by image template matching' }
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
      { name: 'css', type: 'string', description: 'Find by CSS selector' },
      { name: 'xpath', type: 'string', description: 'Find by XPath' },
      { name: 'regex', type: 'string', description: 'Find by regex pattern' },
      { name: 'index', type: 'number', description: 'Element index (0-based)' },
      { name: 'type', type: 'string', description: 'Element type' }
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
    snippet: 'press: "${1|Enter,Back,Home|}"'
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
    snippet: 'scrollTo:\n    ${1|text,id,regex|}: "$2"\n    direction: "${3|down,up|}"',
    params: [
      { name: 'text', type: 'string', description: 'Find by exact text' },
      { name: 'id', type: 'string', description: 'Find by resource ID' },
      { name: 'css', type: 'string', description: 'Find by CSS selector' },
      { name: 'xpath', type: 'string', description: 'Find by XPath' },
      { name: 'regex', type: 'string', description: 'Find by regex pattern' },
      { name: 'direction', type: 'string', description: 'Scroll direction: up, down, left, right' },
      { name: 'maxScrolls', type: 'number', description: 'Maximum scroll attempts' },
      { name: 'image', type: 'string', description: 'Find by image template' }
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
      { name: 'image', type: 'string', description: 'Find by image template' },
      { name: 'timeout', type: 'number', description: 'Wait timeout in ms' }
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
      { name: 'timeout', type: 'number', description: 'Wait timeout in ms' }
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
      { name: 'timeout', type: 'number', description: 'Wait timeout in ms (default: 5000)' }
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
      { name: 'timeout', type: 'number', description: 'Wait timeout in ms (default: 10000)' }
    ]
  },
  {
    name: 'assert',
    aliases: ['assertTrue'],
    category: 'Assertions',
    description: 'Assert a condition is true',
    hasParams: true,
    snippet: 'assert:\n    condition: "${1:\\${count} > 5}"'
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
    snippet: 'assertColor:\n    point: "${1:50%,50%}"\n    color: "${2:#FF0000}"'
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
    snippet: 'runScript: "$1"'
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
    snippet: 'httpRequest:\n    url: "$1"\n    method: "${2|GET,POST,PUT,DELETE|}"'
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
    snippet: 'gps:\n    file: "$1.gpx"\n    speed: ${2:40}'
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
    description: 'Change device locale',
    hasParams: true,
    snippet: 'setLocale: "${1:en_US}"'
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
  }
];
