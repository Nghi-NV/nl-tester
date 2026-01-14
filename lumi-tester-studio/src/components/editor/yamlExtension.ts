import { EditorExtension, extensionRegistry } from './extensions';

/**
 * Lumi Tester Command Completions
 * Based on lumi-tester/src/parser/types.rs and yaml.rs
 */
export interface NexusCommand {
  label: string;
  detail: string;
  documentation?: string;
  insertText: string;
  category: string;
  isConfig?: boolean; // For config section (before steps)
}

// Config/Header completions (in config section, before ---)
export const configCommands: NexusCommand[] = [
  { 
    label: 'appId', 
    detail: 'Application ID (Android package or iOS bundle)', 
    insertText: 'appId: ${1:org.example.app}', 
    category: 'Config', 
    isConfig: true 
  },
  { 
    label: 'url', 
    detail: 'Base URL for web testing', 
    insertText: 'url: ${1:https://example.com}', 
    category: 'Config', 
    isConfig: true 
  },
  { 
    label: 'platform', 
    detail: 'Target platform (android, ios, web)', 
    insertText: 'platform: ${1|android,ios,web|}', 
    category: 'Config', 
    isConfig: true 
  },
  { 
    label: 'tags', 
    detail: 'Tags for this test', 
    insertText: 'tags:\n  - ${1:tag1}\n  - ${2:tag2}', 
    category: 'Config', 
    isConfig: true 
  },
  { 
    label: 'env', 
    detail: 'Environment variables', 
    insertText: 'env:\n  ${1:KEY}: ${2:value}', 
    category: 'Config', 
    isConfig: true 
  },
  { 
    label: 'data', 
    detail: 'Test data', 
    insertText: 'data: ${1:data}', 
    category: 'Config', 
    isConfig: true 
  },
  { 
    label: 'defaultTimeout', 
    detail: 'Default timeout in milliseconds', 
    insertText: 'defaultTimeout: ${1:5000}', 
    category: 'Config', 
    isConfig: true 
  },
];

// All Lumi Tester commands organized by category
export const nexusCommands: NexusCommand[] = [
  // ========== App Lifecycle ==========
  {
    label: 'launchApp',
    detail: 'Launch an application',
    documentation: 'Launch app with optional clearState, clearKeychain, stopApp, permissions',
    insertText: 'launchApp:\n  ${1|appId,clearState,clearKeychain,stopApp,permissions|}: ${2:value}',
    category: 'App Lifecycle'
  },
  {
    label: 'open',
    detail: 'Alias for launchApp',
    documentation: 'Launch app (alias for launchApp)',
    insertText: 'open:\n  ${1|appId,clearState,clearKeychain,stopApp,permissions|}: ${2:value}',
    category: 'App Lifecycle'
  },
  {
    label: 'stopApp',
    detail: 'Stop the application',
    documentation: 'Stop the currently running app',
    insertText: 'stopApp',
    category: 'App Lifecycle'
  },
  {
    label: 'stop',
    detail: 'Alias for stopApp',
    insertText: 'stop',
    category: 'App Lifecycle'
  },

  // ========== Interactions ==========
  {
    label: 'tapOn',
    detail: 'Tap on an element',
    documentation: 'Tap on element by text, id, type, regex, css, xpath, role, placeholder, point, image, or relative position',
    insertText: 'tapOn:\n  ${1|text,id,type,regex,css,xpath,role,placeholder,point,image,optional,exact,rightOf,leftOf,above,below|}: ${2:value}',
    category: 'Interactions'
  },
  {
    label: 'tap',
    detail: 'Alias for tapOn',
    insertText: 'tap:\n  ${1|text,id,type,regex,css,xpath,role,placeholder,point,image,optional,exact|}: ${2:value}',
    category: 'Interactions'
  },
  {
    label: 'longPressOn',
    detail: 'Long press on an element',
    documentation: 'Long press on element',
    insertText: 'longPressOn:\n  ${1|text,id,type,point|}: ${2:value}',
    category: 'Interactions'
  },
  {
    label: 'longPress',
    detail: 'Alias for longPressOn',
    insertText: 'longPress:\n  ${1|text,id,type,point|}: ${2:value}',
    category: 'Interactions'
  },
  {
    label: 'doubleTapOn',
    detail: 'Double tap on an element',
    documentation: 'Double tap on element',
    insertText: 'doubleTapOn:\n  ${1|text,id,type,point|}: ${2:value}',
    category: 'Interactions'
  },
  {
    label: 'doubleTap',
    detail: 'Alias for doubleTapOn',
    insertText: 'doubleTap:\n  ${1|text,id,type,point|}: ${2:value}',
    category: 'Interactions'
  },
  {
    label: 'rightClick',
    detail: 'Right click on an element',
    documentation: 'Right click (context click) on element',
    insertText: 'rightClick:\n  ${1|text,id,type,css|}: ${2:value}',
    category: 'Interactions'
  },
  {
    label: 'contextClick',
    detail: 'Alias for rightClick',
    insertText: 'contextClick:\n  ${1|text,id,type,css|}: ${2:value}',
    category: 'Interactions'
  },
  {
    label: 'inputText',
    detail: 'Input text into the active field',
    documentation: 'Input text (can be string or TypeParams for web)',
    insertText: 'inputText: ${1:"text"}',
    category: 'Interactions'
  },
  {
    label: 'write',
    detail: 'Alias for inputText',
    insertText: 'write: ${1:"text"}',
    category: 'Interactions'
  },
  {
    label: 'type',
    detail: 'Type text (web)',
    documentation: 'Type text with selector (web)',
    insertText: 'type:\n  text: ${1:"text"}\n  selector: ${2:"selector"}',
    category: 'Interactions'
  },
  {
    label: 'eraseText',
    detail: 'Erase text from active field',
    documentation: 'Erase text (optionally specify charCount)',
    insertText: 'eraseText:\n  charCount: ${1:10}',
    category: 'Interactions'
  },
  {
    label: 'clear',
    detail: 'Alias for eraseText',
    insertText: 'clear:\n  charCount: ${1:10}',
    category: 'Interactions'
  },
  {
    label: 'hideKeyboard',
    detail: 'Hide the keyboard',
    insertText: 'hideKeyboard',
    category: 'Interactions'
  },
  {
    label: 'hideKbd',
    detail: 'Alias for hideKeyboard',
    insertText: 'hideKbd',
    category: 'Interactions'
  },
  {
    label: 'tapAt',
    detail: 'Tap element by type and index',
    documentation: 'Tap element by type and index (e.g., tap 2nd EditText)',
    insertText: 'tapAt:\n  type: ${1:"EditText"}\n  index: ${2:0}',
    category: 'Interactions'
  },
  {
    label: 'inputAt',
    detail: 'Input text at element by type and index',
    documentation: 'Input text at element by type and index',
    insertText: 'inputAt:\n  type: ${1:"EditText"}\n  index: ${2:0}\n  text: ${3:"text"}',
    category: 'Interactions'
  },

  // ========== Swipe/Scroll ==========
  {
    label: 'swipeLeft',
    detail: 'Swipe left',
    insertText: 'swipeLeft',
    category: 'Swipe/Scroll'
  },
  {
    label: 'swipeRight',
    detail: 'Swipe right',
    insertText: 'swipeRight',
    category: 'Swipe/Scroll'
  },
  {
    label: 'swipeUp',
    detail: 'Swipe up',
    insertText: 'swipeUp',
    category: 'Swipe/Scroll'
  },
  {
    label: 'swipeDown',
    detail: 'Swipe down',
    insertText: 'swipeDown',
    category: 'Swipe/Scroll'
  },
  {
    label: 'swipe',
    detail: 'Manual scroll',
    documentation: 'Manual scroll with direction, distance, duration, from',
    insertText: 'swipe:\n  direction: ${1|up,down,left,right|}\n  distance: ${2:100}\n  duration: ${3:300}',
    category: 'Swipe/Scroll'
  },
  {
    label: 'scrollUntilVisible',
    detail: 'Scroll until element is visible',
    documentation: 'Scroll until element is visible',
    insertText: 'scrollUntilVisible:\n  ${1|text,id,regex,type,image|}: ${2:value}\n  maxScrolls: ${3:10}',
    category: 'Swipe/Scroll'
  },
  {
    label: 'scrollTo',
    detail: 'Alias for scrollUntilVisible',
    insertText: 'scrollTo:\n  ${1|text,id,regex,type,image|}: ${2:value}\n  maxScrolls: ${3:10}',
    category: 'Swipe/Scroll'
  },

  // ========== Assertions ==========
  {
    label: 'assertVisible',
    detail: 'Assert that element is visible',
    documentation: 'Assert element is visible (can be string or AssertParams)',
    insertText: 'assertVisible:\n  ${1|text,id,regex,type,css,xpath,role,placeholder,image,index,timeout,soft|}: ${2:value}',
    category: 'Assertions'
  },
  {
    label: 'see',
    detail: 'Alias for assertVisible',
    insertText: 'see:\n  ${1|text,id,regex,type,css,xpath,role,placeholder,image,index,timeout,soft|}: ${2:value}',
    category: 'Assertions'
  },
  {
    label: 'assertNotVisible',
    detail: 'Assert that element is not visible',
    documentation: 'Assert element is not visible',
    insertText: 'assertNotVisible:\n  ${1|text,id,regex,type|}: ${2:value}',
    category: 'Assertions'
  },
  {
    label: 'notSee',
    detail: 'Alias for assertNotVisible',
    insertText: 'notSee:\n  ${1|text,id,regex,type|}: ${2:value}',
    category: 'Assertions'
  },
  {
    label: 'waitUntilNotVisible',
    detail: 'Wait until element is not visible',
    documentation: 'Wait until element is not visible',
    insertText: 'waitUntilNotVisible:\n  ${1|text,id,regex|}: ${2:value}',
    category: 'Assertions'
  },
  {
    label: 'waitNotSee',
    detail: 'Alias for waitUntilNotVisible',
    insertText: 'waitNotSee:\n  ${1|text,id,regex|}: ${2:value}',
    category: 'Assertions'
  },
  {
    label: 'assertTrue',
    detail: 'Assert condition is true',
    documentation: 'Assert condition is true (can be expression string or condition object)',
    insertText: 'assertTrue: ${1:"condition"}',
    category: 'Assertions'
  },
  {
    label: 'assert',
    detail: 'Alias for assertTrue',
    insertText: 'assert: ${1:"condition"}',
    category: 'Assertions'
  },
  {
    label: 'assertColor',
    detail: 'Assert color at point',
    documentation: 'Assert color at specific point',
    insertText: 'assertColor:\n  point: ${1:"540,960"}\n  color: ${2:"#4CAF50"}\n  tolerance: ${3:10.0}',
    category: 'Assertions'
  },
  {
    label: 'checkColor',
    detail: 'Alias for assertColor',
    insertText: 'checkColor:\n  point: ${1:"540,960"}\n  color: ${2:"#4CAF50"}\n  tolerance: ${3:10.0}',
    category: 'Assertions'
  },
  {
    label: 'assertScreenshot',
    detail: 'Assert screenshot matches',
    documentation: 'Assert screenshot matches reference',
    insertText: 'assertScreenshot: ${1:"screenshot_name"}',
    category: 'Assertions'
  },

  // ========== Control Flow ==========
  {
    label: 'wait',
    detail: 'Wait for specified time',
    documentation: 'Wait for specified milliseconds (can be number or WaitParams)',
    insertText: 'wait: ${1:1000}',
    category: 'Control Flow'
  },
  {
    label: 'waitForAnimationToEnd',
    detail: 'Wait for animation to end',
    insertText: 'waitForAnimationToEnd',
    category: 'Control Flow'
  },
  {
    label: 'repeat',
    detail: 'Repeat commands',
    documentation: 'Repeat commands (times or while condition)',
    insertText: 'repeat:\n  times: ${1:3}\n  commands:\n    - ${2:command}',
    category: 'Control Flow'
  },
  {
    label: 'retry',
    detail: 'Retry commands on failure',
    documentation: 'Retry commands on failure',
    insertText: 'retry:\n  maxRetries: ${1:3}\n  commands:\n    - ${2:command}',
    category: 'Control Flow'
  },
  {
    label: 'runFlow',
    detail: 'Run another test flow',
    documentation: 'Run another test flow (can be string path or RunFlowParams)',
    insertText: 'runFlow: ${1:"subflows/flow.yaml"}',
    category: 'Control Flow'
  },
  {
    label: 'conditional',
    detail: 'Conditional execution',
    documentation: 'Conditional execution based on visibility',
    insertText: 'conditional:\n  condition:\n    visible: ${1:"text"}\n  then:\n    - ${2:command}\n  else:\n    - ${3:command}',
    category: 'Control Flow'
  },

  // ========== Variables ==========
  {
    label: 'setVar',
    detail: 'Set a variable',
    documentation: 'Set a variable for use in subsequent commands',
    insertText: 'setVar:\n  name: ${1:varName}\n  value: ${2:"value"}',
    category: 'Variables'
  },
  {
    label: 'assertVar',
    detail: 'Assert variable value',
    documentation: 'Assert variable has expected value',
    insertText: 'assertVar:\n  name: ${1:varName}\n  expected: ${2:"value"}',
    category: 'Variables'
  },

  // ========== Media ==========
  {
    label: 'takeScreenshot',
    detail: 'Take a screenshot',
    documentation: 'Take a screenshot (can be string path or ScreenshotParams)',
    insertText: 'takeScreenshot: ${1:"screenshot.png"}',
    category: 'Media'
  },
  {
    label: 'screenshot',
    detail: 'Alias for takeScreenshot',
    insertText: 'screenshot: ${1:"screenshot.png"}',
    category: 'Media'
  },
  {
    label: 'startRecording',
    detail: 'Start video recording',
    documentation: 'Start video recording (can be string path or RecordingParams)',
    insertText: 'startRecording: ${1:"recording.mp4"}',
    category: 'Media'
  },
  {
    label: 'stopRecording',
    detail: 'Stop video recording',
    insertText: 'stopRecording',
    category: 'Media'
  },
  {
    label: 'stopRecord',
    detail: 'Alias for stopRecording',
    insertText: 'stopRecord',
    category: 'Media'
  },
  {
    label: 'openLink',
    detail: 'Open a link (deep link)',
    documentation: 'Open a link or deep link',
    insertText: 'openLink: ${1:"myapp://path"}',
    category: 'Media'
  },
  {
    label: 'deepLink',
    detail: 'Alias for openLink',
    insertText: 'deepLink: ${1:"myapp://path"}',
    category: 'Media'
  },
  {
    label: 'exportReport',
    detail: 'Export test report',
    documentation: 'Export test report',
    insertText: 'exportReport:\n  path: ${1:"report.json"}\n  format: ${2|json,xml,html|}',
    category: 'Media'
  },

  // ========== Navigation ==========
  {
    label: 'back',
    detail: 'Press back button',
    insertText: 'back',
    category: 'Navigation'
  },
  {
    label: 'pressHome',
    detail: 'Press home button',
    insertText: 'pressHome',
    category: 'Navigation'
  },
  {
    label: 'home',
    detail: 'Alias for pressHome',
    insertText: 'home',
    category: 'Navigation'
  },
  {
    label: 'navigate',
    detail: 'Navigate to URL (web)',
    documentation: 'Navigate to URL (web)',
    insertText: 'navigate:\n  url: ${1:"https://example.com"}',
    category: 'Navigation'
  },
  {
    label: 'click',
    detail: 'Click element (web)',
    documentation: 'Click element (web)',
    insertText: 'click:\n  ${1|selector,text|}: ${2:value}',
    category: 'Navigation'
  },

  // ========== Advanced Features ==========
  {
    label: 'generate',
    detail: 'Generate random data',
    documentation: 'Generate random data (uuid, email, phone, name, address, number, date)',
    insertText: 'generate:\n  name: ${1:varName}\n  type: ${2|uuid,email,phone,name,address,number,date|}\n  format: ${3:optional}',
    category: 'Advanced'
  },
  {
    label: 'httpRequest',
    detail: 'Make HTTP request',
    documentation: 'Make HTTP request',
    insertText: 'httpRequest:\n  url: ${1:"https://api.example.com"}\n  method: ${2|GET,POST,PUT,DELETE|}\n  headers:\n    Content-Type: application/json\n  body:\n    ${3:key}: ${4:value}',
    category: 'Advanced'
  },
  {
    label: 'runScript',
    detail: 'Run a script',
    documentation: 'Run a script (can be string command or RunScriptParams)',
    insertText: 'runScript: ${1:"scripts/script.js"}',
    category: 'Advanced'
  },
  {
    label: 'evalScript',
    detail: 'Evaluate JavaScript expression',
    documentation: 'Evaluate JavaScript expression',
    insertText: 'evalScript: ${1:"expression"}',
    category: 'Advanced'
  },

  // ========== GPS Mock Location ==========
  {
    label: 'mockLocation',
    detail: 'Mock GPS location',
    documentation: 'Mock GPS location from GPX/KML/JSON file',
    insertText: 'mockLocation: ${1:"routes/route.gpx"}',
    category: 'GPS'
  },
  {
    label: 'gps',
    detail: 'Alias for mockLocation',
    insertText: 'gps: ${1:"routes/route.gpx"}',
    category: 'GPS'
  },
  {
    label: 'stopMockLocation',
    detail: 'Stop mock location',
    insertText: 'stopMockLocation',
    category: 'GPS'
  },
  {
    label: 'stopGps',
    detail: 'Alias for stopMockLocation',
    insertText: 'stopGps',
    category: 'GPS'
  },
  {
    label: 'mockLocationControl',
    detail: 'Control mock location',
    documentation: 'Control mock location (speed, pause, resume)',
    insertText: 'mockLocationControl:\n  speed: ${1:50.0}\n  speedMode: ${2|linear,noise|}\n  speedNoise: ${3:5.0}',
    category: 'GPS'
  },
  {
    label: 'waitForLocation',
    detail: 'Wait for location',
    documentation: 'Wait for location to reach specific coordinates',
    insertText: 'waitForLocation:\n  lat: ${1:10.0}\n  lon: ${2:106.0}\n  tolerance: ${3:50.0}\n  timeout: ${4:5000}',
    category: 'GPS'
  },
  {
    label: 'waitForMockCompletion',
    detail: 'Wait for mock location to complete',
    documentation: 'Wait for mock location playback to complete',
    insertText: 'waitForMockCompletion:\n  timeout: ${1:5000}',
    category: 'GPS'
  },

  // ========== GIF Recording ==========
  {
    label: 'captureGifFrame',
    detail: 'Capture GIF frame',
    documentation: 'Capture a frame for GIF',
    insertText: 'captureGifFrame: ${1:"frame_name"}',
    category: 'GIF'
  },
  {
    label: 'captureFrame',
    detail: 'Alias for captureGifFrame',
    insertText: 'captureFrame: ${1:"frame_name"}',
    category: 'GIF'
  },
  {
    label: 'buildGif',
    detail: 'Build GIF from frames',
    documentation: 'Build GIF from captured frames',
    insertText: 'buildGif:\n  frames:\n    - ${1:"frame1"}\n    - ${2:"frame2"}\n  output: ${3:"output.gif"}\n  delay: ${4:500}\n  quality: ${5|low,medium,high|}',
    category: 'GIF'
  },
  {
    label: 'createGif',
    detail: 'Alias for buildGif',
    insertText: 'createGif:\n  frames:\n    - ${1:"frame1"}\n    - ${2:"frame2"}\n  output: ${3:"output.gif"}',
    category: 'GIF'
  },
  {
    label: 'startGifCapture',
    detail: 'Start auto-capture GIF mode',
    documentation: 'Start auto-capture GIF mode',
    insertText: 'startGifCapture:\n  interval: ${1:200}\n  maxFrames: ${2:150}\n  width: ${3:optional}',
    category: 'GIF'
  },
  {
    label: 'stopGifCapture',
    detail: 'Stop auto-capture and build GIF',
    documentation: 'Stop auto-capture and build GIF',
    insertText: 'stopGifCapture:\n  output: ${1:"output.gif"}\n  delay: ${2:200}\n  quality: ${3|low,medium,high|}',
    category: 'GIF'
  },

  // ========== Device Control ==========
  {
    label: 'rotateScreen',
    detail: 'Rotate screen',
    documentation: 'Rotate screen (can be string mode or RotationParams)',
    insertText: 'rotateScreen:\n  mode: ${1|portrait,landscape|}',
    category: 'Device Control'
  },
  {
    label: 'rotate',
    detail: 'Alias for rotateScreen',
    insertText: 'rotate: ${1|portrait,landscape|}',
    category: 'Device Control'
  },
  {
    label: 'pressKey',
    detail: 'Press a specific key',
    documentation: 'Press a specific key',
    insertText: 'pressKey: ${1:"KEYCODE_ENTER"}',
    category: 'Device Control'
  },
  {
    label: 'press',
    detail: 'Alias for pressKey',
    insertText: 'press: ${1:"KEYCODE_ENTER"}',
    category: 'Device Control'
  },
  {
    label: 'setOrientation',
    detail: 'Set device orientation',
    documentation: 'Set device orientation',
    insertText: 'setOrientation:\n  mode: ${1|PORTRAIT,LANDSCAPE,LANDSCAPE_LEFT,LANDSCAPE_RIGHT,UPSIDE_DOWN|}',
    category: 'Device Control'
  },
  {
    label: 'setVolume',
    detail: 'Set volume level',
    documentation: 'Set volume level (0-100)',
    insertText: 'setVolume: ${1:50}',
    category: 'Device Control'
  },
  {
    label: 'lockDevice',
    detail: 'Lock the device',
    insertText: 'lockDevice',
    category: 'Device Control'
  },
  {
    label: 'unlockDevice',
    detail: 'Unlock the device',
    insertText: 'unlockDevice',
    category: 'Device Control'
  },
  {
    label: 'openNotifications',
    detail: 'Open notifications panel',
    insertText: 'openNotifications',
    category: 'Device Control'
  },
  {
    label: 'openQuickSettings',
    detail: 'Open quick settings panel',
    insertText: 'openQuickSettings',
    category: 'Device Control'
  },

  // ========== File Management ==========
  {
    label: 'pushFile',
    detail: 'Push file to device',
    documentation: 'Push file to device',
    insertText: 'pushFile:\n  source: ${1:"local/path"}\n  destination: ${2:"/sdcard/path"}',
    category: 'File Management'
  },
  {
    label: 'pullFile',
    detail: 'Pull file from device',
    documentation: 'Pull file from device',
    insertText: 'pullFile:\n  source: ${1:"/sdcard/path"}\n  destination: ${2:"local/path"}',
    category: 'File Management'
  },
  {
    label: 'clearAppData',
    detail: 'Clear app data',
    documentation: 'Clear app data by package ID',
    insertText: 'clearAppData: ${1:"org.example.app"}',
    category: 'File Management'
  },

  // ========== Clipboard ==========
  {
    label: 'setClipboard',
    detail: 'Set clipboard content',
    documentation: 'Set clipboard content',
    insertText: 'setClipboard: ${1:"text"}',
    category: 'Clipboard'
  },
  {
    label: 'getClipboard',
    detail: 'Get clipboard content',
    documentation: 'Get clipboard content and save to variable',
    insertText: 'getClipboard:\n  name: ${1:varName}',
    category: 'Clipboard'
  },
  {
    label: 'assertClipboard',
    detail: 'Assert clipboard content',
    documentation: 'Assert clipboard has expected content',
    insertText: 'assertClipboard: ${1:"expected_text"}',
    category: 'Clipboard'
  },
  {
    label: 'copyTextFrom',
    detail: 'Copy text from element',
    documentation: 'Copy text from element',
    insertText: 'copyTextFrom:\n  ${1|text,id|}: ${2:value}',
    category: 'Clipboard'
  },
  {
    label: 'pasteText',
    detail: 'Paste text from clipboard',
    insertText: 'pasteText',
    category: 'Clipboard'
  },

  // ========== Random Input ==========
  {
    label: 'inputRandomEmail',
    detail: 'Input random email',
    insertText: 'inputRandomEmail',
    category: 'Random Input'
  },
  {
    label: 'inputRandomNumber',
    detail: 'Input random number',
    documentation: 'Input random number (optionally specify length)',
    insertText: 'inputRandomNumber:\n  length: ${1:10}',
    category: 'Random Input'
  },
  {
    label: 'inputRandomPhoneNumber',
    detail: 'Alias for inputRandomNumber',
    insertText: 'inputRandomPhoneNumber:\n  length: ${1:10}',
    category: 'Random Input'
  },
  {
    label: 'inputRandomPersonName',
    detail: 'Input random person name',
    insertText: 'inputRandomPersonName',
    category: 'Random Input'
  },
  {
    label: 'inputRandomText',
    detail: 'Input random text',
    documentation: 'Input random text (optionally specify length)',
    insertText: 'inputRandomText:\n  length: ${1:10}',
    category: 'Random Input'
  },

  // ========== Extended Wait ==========
  {
    label: 'extendedWaitUntil',
    detail: 'Extended wait until condition',
    documentation: 'Extended wait until condition with timeout',
    insertText: 'extendedWaitUntil:\n  timeout: ${1:5000}\n  visible:\n    text: ${2:"text"}\n  notVisible:\n    text: ${3:"text"}',
    category: 'Extended Wait'
  },

  // ========== Database ==========
  {
    label: 'dbQuery',
    detail: 'Execute database query',
    documentation: 'Execute database query',
    insertText: 'dbQuery:\n  connection: ${1:"db_name"}\n  query: ${2:"SELECT * FROM table"}\n  params:\n    - ${3:param1}\n  save:\n    ${4:key}: ${5:jsonPath}',
    category: 'Database'
  },

  // ========== Network & Connectivity ==========
  {
    label: 'setNetwork',
    detail: 'Set network state',
    documentation: 'Set network state (wifi, data)',
    insertText: 'setNetwork:\n  wifi: ${1|true,false|}\n  data: ${2|true,false|}',
    category: 'Network'
  },
  {
    label: 'toggleAirplaneMode',
    detail: 'Toggle airplane mode',
    insertText: 'toggleAirplaneMode',
    category: 'Network'
  },
  {
    label: 'airplaneMode',
    detail: 'Alias for toggleAirplaneMode',
    insertText: 'airplaneMode',
    category: 'Network'
  },

  // ========== App Management ==========
  {
    label: 'installApp',
    detail: 'Install app',
    documentation: 'Install app from path',
    insertText: 'installApp: ${1:"path/to/app.apk"}',
    category: 'App Management'
  },
  {
    label: 'uninstallApp',
    detail: 'Uninstall app',
    documentation: 'Uninstall app by package ID',
    insertText: 'uninstallApp: ${1:"org.example.app"}',
    category: 'App Management'
  },
  {
    label: 'backgroundApp',
    detail: 'Background app',
    documentation: 'Background app for specified duration',
    insertText: 'backgroundApp:\n  appId: ${1:"org.example.app"}\n  durationMs: ${2:5000}',
    category: 'App Management'
  },
];

export const CONFIG_KEYWORDS = [
  { label: 'appId', documentation: 'Application ID (Android package or iOS bundle)', insertText: 'appId: ' },
  { label: 'url', documentation: 'Base URL for web testing', insertText: 'url: ' },
  { label: 'platform', documentation: 'Target platform (android, ios, web)', insertText: 'platform: ' },
  { label: 'tags', documentation: 'Tags for this test', insertText: 'tags:\n  - ' },
  { label: 'env', documentation: 'Environment variables', insertText: 'env:\n  KEY: value' },
  { label: 'data', documentation: 'Test data', insertText: 'data: ' },
  { label: 'defaultTimeout', documentation: 'Default timeout in milliseconds', insertText: 'defaultTimeout: ' },
];

/**
 * Command properties map - properties available for each command
 * Based on lumi-tester/src/parser/types.rs
 */
export interface CommandProperty {
  label: string;
  detail: string;
  documentation?: string;
  insertText: string;
  type?: string; // 'string', 'number', 'boolean', 'object', 'array'
}

export const commandProperties: Record<string, CommandProperty[]> = {
  // TapParams - used by tapOn, longPressOn, doubleTapOn, rightClick
  tapOn: [
    { label: 'text', detail: 'Text content to match', insertText: 'text: ${1:"text"}', type: 'string' },
    { label: 'id', detail: 'Element ID/resource-id', insertText: 'id: ${1:"id"}', type: 'string' },
    { label: 'type', detail: 'Element type/class (e.g., EditText, Button)', insertText: 'type: ${1:"EditText"}', type: 'string' },
    { label: 'regex', detail: 'Regular expression to match', insertText: 'regex: ${1:"pattern"}', type: 'string' },
    { label: 'css', detail: 'CSS selector (web)', insertText: 'css: ${1:"selector"}', type: 'string' },
    { label: 'xpath', detail: 'XPath selector', insertText: 'xpath: ${1:"//path"}', type: 'string' },
    { label: 'role', detail: 'Accessibility role', insertText: 'role: ${1:"button"}', type: 'string' },
    { label: 'placeholder', detail: 'Placeholder text', insertText: 'placeholder: ${1:"placeholder"}', type: 'string' },
    { label: 'point', detail: 'Tap at coordinates (x,y)', insertText: 'point: ${1:"540,960"}', type: 'string' },
    { label: 'image', detail: 'Image template path', insertText: 'image: ${1:"path/to/image.png"}', type: 'string' },
    { label: 'index', detail: 'Element index (0-based)', insertText: 'index: ${1:0}', type: 'number' },
    { label: 'optional', detail: 'Make this step optional', insertText: 'optional: ${1|true,false|}', type: 'boolean' },
    { label: 'exact', detail: 'Require exact text match (case-sensitive)', insertText: 'exact: ${1|true,false|}', type: 'boolean' },
    { label: 'rightOf', detail: 'Element to the right of', insertText: 'rightOf: ${1:"text"}', type: 'string' },
    { label: 'leftOf', detail: 'Element to the left of', insertText: 'leftOf: ${1:"text"}', type: 'string' },
    { label: 'above', detail: 'Element above', insertText: 'above: ${1:"text"}', type: 'string' },
    { label: 'below', detail: 'Element below', insertText: 'below: ${1:"text"}', type: 'string' },
  ],
  tap: [
    { label: 'text', detail: 'Text content to match', insertText: 'text: ${1:"text"}', type: 'string' },
    { label: 'id', detail: 'Element ID/resource-id', insertText: 'id: ${1:"id"}', type: 'string' },
    { label: 'type', detail: 'Element type/class', insertText: 'type: ${1:"EditText"}', type: 'string' },
    { label: 'regex', detail: 'Regular expression', insertText: 'regex: ${1:"pattern"}', type: 'string' },
    { label: 'css', detail: 'CSS selector', insertText: 'css: ${1:"selector"}', type: 'string' },
    { label: 'xpath', detail: 'XPath selector', insertText: 'xpath: ${1:"//path"}', type: 'string' },
    { label: 'role', detail: 'Accessibility role', insertText: 'role: ${1:"button"}', type: 'string' },
    { label: 'placeholder', detail: 'Placeholder text', insertText: 'placeholder: ${1:"placeholder"}', type: 'string' },
    { label: 'point', detail: 'Tap at coordinates', insertText: 'point: ${1:"540,960"}', type: 'string' },
    { label: 'image', detail: 'Image template path', insertText: 'image: ${1:"path/to/image.png"}', type: 'string' },
    { label: 'optional', detail: 'Make step optional', insertText: 'optional: ${1|true,false|}', type: 'boolean' },
    { label: 'exact', detail: 'Exact text match', insertText: 'exact: ${1|true,false|}', type: 'boolean' },
  ],
  longPressOn: [
    { label: 'text', detail: 'Text content to match', insertText: 'text: ${1:"text"}', type: 'string' },
    { label: 'id', detail: 'Element ID/resource-id', insertText: 'id: ${1:"id"}', type: 'string' },
    { label: 'type', detail: 'Element type/class', insertText: 'type: ${1:"EditText"}', type: 'string' },
    { label: 'point', detail: 'Long press at coordinates', insertText: 'point: ${1:"540,960"}', type: 'string' },
  ],
  longPress: [
    { label: 'text', detail: 'Text content to match', insertText: 'text: ${1:"text"}', type: 'string' },
    { label: 'id', detail: 'Element ID/resource-id', insertText: 'id: ${1:"id"}', type: 'string' },
    { label: 'type', detail: 'Element type/class', insertText: 'type: ${1:"EditText"}', type: 'string' },
    { label: 'point', detail: 'Long press at coordinates', insertText: 'point: ${1:"540,960"}', type: 'string' },
  ],
  doubleTapOn: [
    { label: 'text', detail: 'Text content to match', insertText: 'text: ${1:"text"}', type: 'string' },
    { label: 'id', detail: 'Element ID/resource-id', insertText: 'id: ${1:"id"}', type: 'string' },
    { label: 'type', detail: 'Element type/class', insertText: 'type: ${1:"EditText"}', type: 'string' },
    { label: 'point', detail: 'Double tap at coordinates', insertText: 'point: ${1:"540,960"}', type: 'string' },
  ],
  doubleTap: [
    { label: 'text', detail: 'Text content to match', insertText: 'text: ${1:"text"}', type: 'string' },
    { label: 'id', detail: 'Element ID/resource-id', insertText: 'id: ${1:"id"}', type: 'string' },
    { label: 'type', detail: 'Element type/class', insertText: 'type: ${1:"EditText"}', type: 'string' },
    { label: 'point', detail: 'Double tap at coordinates', insertText: 'point: ${1:"540,960"}', type: 'string' },
  ],
  rightClick: [
    { label: 'text', detail: 'Text content to match', insertText: 'text: ${1:"text"}', type: 'string' },
    { label: 'id', detail: 'Element ID/resource-id', insertText: 'id: ${1:"id"}', type: 'string' },
    { label: 'type', detail: 'Element type/class', insertText: 'type: ${1:"EditText"}', type: 'string' },
    { label: 'css', detail: 'CSS selector', insertText: 'css: ${1:"selector"}', type: 'string' },
  ],
  contextClick: [
    { label: 'text', detail: 'Text content to match', insertText: 'text: ${1:"text"}', type: 'string' },
    { label: 'id', detail: 'Element ID/resource-id', insertText: 'id: ${1:"id"}', type: 'string' },
    { label: 'type', detail: 'Element type/class', insertText: 'type: ${1:"EditText"}', type: 'string' },
    { label: 'css', detail: 'CSS selector', insertText: 'css: ${1:"selector"}', type: 'string' },
  ],
  eraseText: [
    { label: 'charCount', detail: 'Number of characters to erase', insertText: 'charCount: ${1:10}', type: 'number' },
  ],
  clear: [
    { label: 'charCount', detail: 'Number of characters to erase', insertText: 'charCount: ${1:10}', type: 'number' },
  ],
  tapAt: [
    { label: 'type', detail: 'Element type/class (required)', insertText: 'type: ${1:"EditText"}', type: 'string' },
    { label: 'index', detail: 'Element index (0-based)', insertText: 'index: ${1:0}', type: 'number' },
  ],
  inputAt: [
    { label: 'type', detail: 'Element type/class (required)', insertText: 'type: ${1:"EditText"}', type: 'string' },
    { label: 'index', detail: 'Element index (0-based)', insertText: 'index: ${1:0}', type: 'number' },
    { label: 'text', detail: 'Text to input (required)', insertText: 'text: ${1:"text"}', type: 'string' },
  ],
  swipe: [
    { label: 'direction', detail: 'Swipe direction', insertText: 'direction: ${1|up,down,left,right|}', type: 'string' },
    { label: 'distance', detail: 'Swipe distance in pixels', insertText: 'distance: ${1:100}', type: 'number' },
    { label: 'duration', detail: 'Swipe duration in milliseconds', insertText: 'duration: ${1:300}', type: 'number' },
    { label: 'from', detail: 'Start from element', insertText: 'from:\n    text: ${1:"text"}', type: 'object' },
  ],
  scrollUntilVisible: [
    { label: 'text', detail: 'Text to scroll to', insertText: 'text: ${1:"text"}', type: 'string' },
    { label: 'id', detail: 'Element ID to scroll to', insertText: 'id: ${1:"id"}', type: 'string' },
    { label: 'regex', detail: 'Regex pattern to scroll to', insertText: 'regex: ${1:"pattern"}', type: 'string' },
    { label: 'type', detail: 'Element type to scroll to', insertText: 'type: ${1:"EditText"}', type: 'string' },
    { label: 'image', detail: 'Image template to scroll to', insertText: 'image: ${1:"path/to/image.png"}', type: 'string' },
    { label: 'maxScrolls', detail: 'Maximum number of scrolls', insertText: 'maxScrolls: ${1:10}', type: 'number' },
    { label: 'direction', detail: 'Scroll direction', insertText: 'direction: ${1|up,down,left,right|}', type: 'string' },
  ],
  scrollTo: [
    { label: 'text', detail: 'Text to scroll to', insertText: 'text: ${1:"text"}', type: 'string' },
    { label: 'id', detail: 'Element ID to scroll to', insertText: 'id: ${1:"id"}', type: 'string' },
    { label: 'regex', detail: 'Regex pattern to scroll to', insertText: 'regex: ${1:"pattern"}', type: 'string' },
    { label: 'type', detail: 'Element type to scroll to', insertText: 'type: ${1:"EditText"}', type: 'string' },
    { label: 'image', detail: 'Image template to scroll to', insertText: 'image: ${1:"path/to/image.png"}', type: 'string' },
    { label: 'maxScrolls', detail: 'Maximum number of scrolls', insertText: 'maxScrolls: ${1:10}', type: 'number' },
    { label: 'direction', detail: 'Scroll direction', insertText: 'direction: ${1|up,down,left,right|}', type: 'string' },
  ],
  // AssertParams - used by assertVisible, assertNotVisible, waitUntilNotVisible
  assertVisible: [
    { label: 'text', detail: 'Text content to assert', insertText: 'text: ${1:"text"}', type: 'string' },
    { label: 'id', detail: 'Element ID to assert', insertText: 'id: ${1:"id"}', type: 'string' },
    { label: 'regex', detail: 'Regex pattern to assert', insertText: 'regex: ${1:"pattern"}', type: 'string' },
    { label: 'type', detail: 'Element type to assert', insertText: 'type: ${1:"EditText"}', type: 'string' },
    { label: 'css', detail: 'CSS selector to assert', insertText: 'css: ${1:"selector"}', type: 'string' },
    { label: 'xpath', detail: 'XPath selector to assert', insertText: 'xpath: ${1:"//path"}', type: 'string' },
    { label: 'role', detail: 'Accessibility role to assert', insertText: 'role: ${1:"button"}', type: 'string' },
    { label: 'placeholder', detail: 'Placeholder text to assert', insertText: 'placeholder: ${1:"placeholder"}', type: 'string' },
    { label: 'image', detail: 'Image template to assert', insertText: 'image: ${1:"path/to/image.png"}', type: 'string' },
    { label: 'index', detail: 'Element index', insertText: 'index: ${1:0}', type: 'number' },
    { label: 'timeout', detail: 'Timeout in milliseconds', insertText: 'timeout: ${1:5000}', type: 'number' },
    { label: 'soft', detail: 'Soft assertion (does not fail test)', insertText: 'soft: ${1|true,false|}', type: 'boolean' },
    { label: 'rightOf', detail: 'Element to the right of', insertText: 'rightOf: ${1:"text"}', type: 'string' },
    { label: 'leftOf', detail: 'Element to the left of', insertText: 'leftOf: ${1:"text"}', type: 'string' },
    { label: 'above', detail: 'Element above', insertText: 'above: ${1:"text"}', type: 'string' },
    { label: 'below', detail: 'Element below', insertText: 'below: ${1:"text"}', type: 'string' },
  ],
  see: [
    { label: 'text', detail: 'Text content to assert', insertText: 'text: ${1:"text"}', type: 'string' },
    { label: 'id', detail: 'Element ID to assert', insertText: 'id: ${1:"id"}', type: 'string' },
    { label: 'regex', detail: 'Regex pattern to assert', insertText: 'regex: ${1:"pattern"}', type: 'string' },
    { label: 'type', detail: 'Element type to assert', insertText: 'type: ${1:"EditText"}', type: 'string' },
    { label: 'css', detail: 'CSS selector to assert', insertText: 'css: ${1:"selector"}', type: 'string' },
    { label: 'image', detail: 'Image template to assert', insertText: 'image: ${1:"path/to/image.png"}', type: 'string' },
    { label: 'timeout', detail: 'Timeout in milliseconds', insertText: 'timeout: ${1:5000}', type: 'number' },
    { label: 'soft', detail: 'Soft assertion', insertText: 'soft: ${1|true,false|}', type: 'boolean' },
  ],
  assertNotVisible: [
    { label: 'text', detail: 'Text content to assert not visible', insertText: 'text: ${1:"text"}', type: 'string' },
    { label: 'id', detail: 'Element ID to assert not visible', insertText: 'id: ${1:"id"}', type: 'string' },
    { label: 'regex', detail: 'Regex pattern to assert not visible', insertText: 'regex: ${1:"pattern"}', type: 'string' },
    { label: 'type', detail: 'Element type to assert not visible', insertText: 'type: ${1:"EditText"}', type: 'string' },
  ],
  notSee: [
    { label: 'text', detail: 'Text content to assert not visible', insertText: 'text: ${1:"text"}', type: 'string' },
    { label: 'id', detail: 'Element ID to assert not visible', insertText: 'id: ${1:"id"}', type: 'string' },
    { label: 'regex', detail: 'Regex pattern to assert not visible', insertText: 'regex: ${1:"pattern"}', type: 'string' },
  ],
  waitUntilNotVisible: [
    { label: 'text', detail: 'Text to wait until not visible', insertText: 'text: ${1:"text"}', type: 'string' },
    { label: 'id', detail: 'Element ID to wait until not visible', insertText: 'id: ${1:"id"}', type: 'string' },
    { label: 'regex', detail: 'Regex pattern to wait until not visible', insertText: 'regex: ${1:"pattern"}', type: 'string' },
  ],
  waitNotSee: [
    { label: 'text', detail: 'Text to wait until not visible', insertText: 'text: ${1:"text"}', type: 'string' },
    { label: 'id', detail: 'Element ID to wait until not visible', insertText: 'id: ${1:"id"}', type: 'string' },
    { label: 'regex', detail: 'Regex pattern to wait until not visible', insertText: 'regex: ${1:"pattern"}', type: 'string' },
  ],
  wait: [
    { label: 'ms', detail: 'Wait time in milliseconds', insertText: 'ms: ${1:1000}', type: 'number' },
  ],
  launchApp: [
    { label: 'appId', detail: 'Application ID', insertText: 'appId: ${1:"org.example.app"}', type: 'string' },
    { label: 'clearState', detail: 'Clear app state before launch', insertText: 'clearState: ${1|true,false|}', type: 'boolean' },
    { label: 'clearKeychain', detail: 'Clear iOS keychain (simulator only)', insertText: 'clearKeychain: ${1|true,false|}', type: 'boolean' },
    { label: 'stopApp', detail: 'Stop app before launching', insertText: 'stopApp: ${1|true,false|}', type: 'boolean' },
    { label: 'permissions', detail: 'Set permissions', insertText: 'permissions:\n    all: ${1|allow,deny|}', type: 'object' },
  ],
  open: [
    { label: 'appId', detail: 'Application ID', insertText: 'appId: ${1:"org.example.app"}', type: 'string' },
    { label: 'clearState', detail: 'Clear app state before launch', insertText: 'clearState: ${1|true,false|}', type: 'boolean' },
    { label: 'clearKeychain', detail: 'Clear iOS keychain', insertText: 'clearKeychain: ${1|true,false|}', type: 'boolean' },
    { label: 'stopApp', detail: 'Stop app before launching', insertText: 'stopApp: ${1|true,false|}', type: 'boolean' },
    { label: 'permissions', detail: 'Set permissions', insertText: 'permissions:\n    all: ${1|allow,deny|}', type: 'object' },
  ],
  runFlow: [
    { label: 'path', detail: 'Path to flow file', insertText: 'path: ${1:"subflows/flow.yaml"}', type: 'string' },
    { label: 'vars', detail: 'Variables to pass to flow', insertText: 'vars:\n    ${1:key}: ${2:value}', type: 'object' },
    { label: 'env', detail: 'Alias for vars', insertText: 'env:\n    ${1:key}: ${2:value}', type: 'object' },
    { label: 'commands', detail: 'Inline commands', insertText: 'commands:\n    - ${1:command}', type: 'array' },
    { label: 'when', detail: 'Conditional execution', insertText: 'when: ${1:condition}', type: 'object' },
    { label: 'label', detail: 'Step label', insertText: 'label: ${1:"label"}', type: 'string' },
    { label: 'optional', detail: 'Make step optional', insertText: 'optional: ${1|true,false|}', type: 'boolean' },
  ],
  assertColor: [
    { label: 'point', detail: 'Point to check (x,y or x%,y%)', insertText: 'point: ${1:"540,960"}', type: 'string' },
    { label: 'color', detail: 'Expected color (#RRGGBB or named)', insertText: 'color: ${1:"#4CAF50"}', type: 'string' },
    { label: 'tolerance', detail: 'Color tolerance percentage', insertText: 'tolerance: ${1:10.0}', type: 'number' },
  ],
  checkColor: [
    { label: 'point', detail: 'Point to check', insertText: 'point: ${1:"540,960"}', type: 'string' },
    { label: 'color', detail: 'Expected color', insertText: 'color: ${1:"#4CAF50"}', type: 'string' },
    { label: 'tolerance', detail: 'Color tolerance', insertText: 'tolerance: ${1:10.0}', type: 'number' },
  ],
  repeat: [
    { label: 'times', detail: 'Number of times to repeat', insertText: 'times: ${1:3}', type: 'number' },
    { label: 'while', detail: 'While condition', insertText: 'while: ${1:condition}', type: 'object' },
    { label: 'commands', detail: 'Commands to repeat (required)', insertText: 'commands:\n    - ${1:command}', type: 'array' },
  ],
  retry: [
    { label: 'maxRetries', detail: 'Maximum number of retries', insertText: 'maxRetries: ${1:3}', type: 'number' },
    { label: 'commands', detail: 'Commands to retry (required)', insertText: 'commands:\n    - ${1:command}', type: 'array' },
  ],
  setVar: [
    { label: 'name', detail: 'Variable name (required)', insertText: 'name: ${1:varName}', type: 'string' },
    { label: 'value', detail: 'Variable value (required)', insertText: 'value: ${1:"value"}', type: 'string' },
  ],
  assertVar: [
    { label: 'name', detail: 'Variable name (required)', insertText: 'name: ${1:varName}', type: 'string' },
    { label: 'expected', detail: 'Expected value (required)', insertText: 'expected: ${1:"value"}', type: 'string' },
  ],
  generate: [
    { label: 'name', detail: 'Variable name (required)', insertText: 'name: ${1:varName}', type: 'string' },
    { label: 'type', detail: 'Data type (required)', insertText: 'type: ${1|uuid,email,phone,name,address,number,date|}', type: 'string' },
    { label: 'format', detail: 'Format string (optional)', insertText: 'format: ${1:"format"}', type: 'string' },
  ],
  httpRequest: [
    { label: 'url', detail: 'Request URL (required)', insertText: 'url: ${1:"https://api.example.com"}', type: 'string' },
    { label: 'method', detail: 'HTTP method (required)', insertText: 'method: ${1|GET,POST,PUT,DELETE|}', type: 'string' },
    { label: 'headers', detail: 'Request headers', insertText: 'headers:\n    Content-Type: application/json', type: 'object' },
    { label: 'body', detail: 'Request body', insertText: 'body:\n    ${1:key}: ${2:value}', type: 'object' },
    { label: 'saveResponse', detail: 'Save response to variables', insertText: 'saveResponse:\n    ${1:var}: ${2:jsonPath}', type: 'object' },
    { label: 'timeoutMs', detail: 'Request timeout', insertText: 'timeoutMs: ${1:5000}', type: 'number' },
  ],
  runScript: [
    { label: 'command', detail: 'Script command (required)', insertText: 'command: ${1:"scripts/script.js"}', type: 'string' },
    { label: 'args', detail: 'Command arguments', insertText: 'args:\n    - ${1:arg1}', type: 'array' },
    { label: 'saveOutput', detail: 'Save output to variable', insertText: 'saveOutput: ${1:varName}', type: 'string' },
    { label: 'timeoutMs', detail: 'Script timeout', insertText: 'timeoutMs: ${1:5000}', type: 'number' },
    { label: 'failOnError', detail: 'Fail test on error', insertText: 'failOnError: ${1|true,false|}', type: 'boolean' },
  ],
  conditional: [
    { label: 'condition', detail: 'Condition object (required)', insertText: 'condition:\n    visible: ${1:"text"}', type: 'object' },
    { label: 'then', detail: 'Commands to run if true (required)', insertText: 'then:\n    - ${1:command}', type: 'array' },
    { label: 'else', detail: 'Commands to run if false', insertText: 'else:\n    - ${1:command}', type: 'array' },
  ],
  mockLocation: [
    { label: 'file', detail: 'GPX/KML/JSON file path (required)', insertText: 'file: ${1:"routes/route.gpx"}', type: 'string' },
    { label: 'name', detail: 'Mock instance name', insertText: 'name: ${1:"instance1"}', type: 'string' },
    { label: 'speed', detail: 'Override speed in km/h', insertText: 'speed: ${1:50.0}', type: 'number' },
    { label: 'speedMode', detail: 'Speed mode', insertText: 'speedMode: ${1|linear,noise|}', type: 'string' },
    { label: 'speedNoise', detail: 'Speed noise range in km/h', insertText: 'speedNoise: ${1:5.0}', type: 'number' },
    { label: 'loop', detail: 'Loop the route', insertText: 'loop: ${1|true,false|}', type: 'boolean' },
    { label: 'startIndex', detail: 'Start from waypoint index', insertText: 'startIndex: ${1:0}', type: 'number' },
    { label: 'intervalMs', detail: 'Update interval in milliseconds', insertText: 'intervalMs: ${1:1000}', type: 'number' },
  ],
  gps: [
    { label: 'file', detail: 'GPX/KML/JSON file path', insertText: 'file: ${1:"routes/route.gpx"}', type: 'string' },
    { label: 'name', detail: 'Mock instance name', insertText: 'name: ${1:"instance1"}', type: 'string' },
    { label: 'speed', detail: 'Override speed', insertText: 'speed: ${1:50.0}', type: 'number' },
    { label: 'loop', detail: 'Loop the route', insertText: 'loop: ${1|true,false|}', type: 'boolean' },
  ],
  mockLocationControl: [
    { label: 'name', detail: 'Mock instance name', insertText: 'name: ${1:"instance1"}', type: 'string' },
    { label: 'speed', detail: 'New speed in km/h', insertText: 'speed: ${1:50.0}', type: 'number' },
    { label: 'speedMode', detail: 'Speed mode', insertText: 'speedMode: ${1|linear,noise|}', type: 'string' },
    { label: 'speedNoise', detail: 'Speed noise range', insertText: 'speedNoise: ${1:5.0}', type: 'number' },
    { label: 'pause', detail: 'Pause playback', insertText: 'pause: ${1|true,false|}', type: 'boolean' },
    { label: 'resume', detail: 'Resume playback', insertText: 'resume: ${1|true,false|}', type: 'boolean' },
  ],
  waitForLocation: [
    { label: 'name', detail: 'Mock instance name', insertText: 'name: ${1:"instance1"}', type: 'string' },
    { label: 'lat', detail: 'Target latitude (required)', insertText: 'lat: ${1:10.0}', type: 'number' },
    { label: 'lon', detail: 'Target longitude (required)', insertText: 'lon: ${1:106.0}', type: 'number' },
    { label: 'tolerance', detail: 'Tolerance in meters', insertText: 'tolerance: ${1:50.0}', type: 'number' },
    { label: 'timeout', detail: 'Timeout in milliseconds', insertText: 'timeout: ${1:5000}', type: 'number' },
  ],
  waitForMockCompletion: [
    { label: 'name', detail: 'Mock instance name', insertText: 'name: ${1:"instance1"}', type: 'string' },
    { label: 'timeout', detail: 'Timeout in milliseconds', insertText: 'timeout: ${1:5000}', type: 'number' },
  ],
  extendedWaitUntil: [
    { label: 'timeout', detail: 'Timeout in milliseconds (required)', insertText: 'timeout: ${1:5000}', type: 'number' },
    { label: 'visible', detail: 'Element must be visible', insertText: 'visible:\n    text: ${1:"text"}', type: 'object' },
    { label: 'notVisible', detail: 'Element must not be visible', insertText: 'notVisible:\n    text: ${1:"text"}', type: 'object' },
  ],
  copyTextFrom: [
    { label: 'text', detail: 'Text to copy from', insertText: 'text: ${1:"text"}', type: 'string' },
    { label: 'id', detail: 'Element ID to copy from', insertText: 'id: ${1:"id"}', type: 'string' },
    { label: 'index', detail: 'Element index', insertText: 'index: ${1:0}', type: 'number' },
  ],
  type: [
    { label: 'text', detail: 'Text to type (required)', insertText: 'text: ${1:"text"}', type: 'string' },
    { label: 'selector', detail: 'CSS selector', insertText: 'selector: ${1:"selector"}', type: 'string' },
  ],
  click: [
    { label: 'selector', detail: 'CSS selector', insertText: 'selector: ${1:"selector"}', type: 'string' },
    { label: 'text', detail: 'Text to click', insertText: 'text: ${1:"text"}', type: 'string' },
  ],
  navigate: [
    { label: 'url', detail: 'URL to navigate to (required)', insertText: 'url: ${1:"https://example.com"}', type: 'string' },
  ],
  rotateScreen: [
    { label: 'mode', detail: 'Rotation mode', insertText: 'mode: ${1|portrait,landscape|}', type: 'string' },
  ],
  rotate: [
    { label: 'mode', detail: 'Rotation mode', insertText: 'mode: ${1|portrait,landscape|}', type: 'string' },
  ],
  setOrientation: [
    { label: 'mode', detail: 'Orientation mode', insertText: 'mode: ${1|PORTRAIT,LANDSCAPE,LANDSCAPE_LEFT,LANDSCAPE_RIGHT,UPSIDE_DOWN|}', type: 'string' },
  ],
  pushFile: [
    { label: 'source', detail: 'Source file path (required)', insertText: 'source: ${1:"local/path"}', type: 'string' },
    { label: 'destination', detail: 'Destination path (required)', insertText: 'destination: ${1:"/sdcard/path"}', type: 'string' },
  ],
  pullFile: [
    { label: 'source', detail: 'Source file path (required)', insertText: 'source: ${1:"/sdcard/path"}', type: 'string' },
    { label: 'destination', detail: 'Destination path (required)', insertText: 'destination: ${1:"local/path"}', type: 'string' },
  ],
  backgroundApp: [
    { label: 'appId', detail: 'App ID to background', insertText: 'appId: ${1:"org.example.app"}', type: 'string' },
    { label: 'durationMs', detail: 'Background duration in milliseconds', insertText: 'durationMs: ${1:5000}', type: 'number' },
  ],
  setNetwork: [
    { label: 'wifi', detail: 'WiFi enabled', insertText: 'wifi: ${1|true,false|}', type: 'boolean' },
    { label: 'data', detail: 'Mobile data enabled', insertText: 'data: ${1|true,false|}', type: 'boolean' },
  ],
  dbQuery: [
    { label: 'connection', detail: 'Database connection name (required)', insertText: 'connection: ${1:"db_name"}', type: 'string' },
    { label: 'query', detail: 'SQL query (required)', insertText: 'query: ${1:"SELECT * FROM table"}', type: 'string' },
    { label: 'params', detail: 'Query parameters', insertText: 'params:\n    - ${1:param1}', type: 'array' },
    { label: 'save', detail: 'Save results to variables', insertText: 'save:\n    ${1:var}: ${2:jsonPath}', type: 'object' },
  ],
  inputRandomNumber: [
    { label: 'length', detail: 'Number length', insertText: 'length: ${1:10}', type: 'number' },
  ],
  inputRandomText: [
    { label: 'length', detail: 'Text length', insertText: 'length: ${1:10}', type: 'number' },
  ],
  buildGif: [
    { label: 'frames', detail: 'Frame names (required)', insertText: 'frames:\n    - ${1:"frame1"}', type: 'array' },
    { label: 'output', detail: 'Output file path (required)', insertText: 'output: ${1:"output.gif"}', type: 'string' },
    { label: 'delay', detail: 'Frame delay in milliseconds', insertText: 'delay: ${1:500}', type: 'number' },
    { label: 'quality', detail: 'GIF quality', insertText: 'quality: ${1|low,medium,high|}', type: 'string' },
    { label: 'width', detail: 'Resize width', insertText: 'width: ${1:800}', type: 'number' },
    { label: 'height', detail: 'Resize height', insertText: 'height: ${1:600}', type: 'number' },
  ],
  createGif: [
    { label: 'frames', detail: 'Frame names', insertText: 'frames:\n    - ${1:"frame1"}', type: 'array' },
    { label: 'output', detail: 'Output file path', insertText: 'output: ${1:"output.gif"}', type: 'string' },
  ],
  startGifCapture: [
    { label: 'interval', detail: 'Capture interval in milliseconds', insertText: 'interval: ${1:200}', type: 'number' },
    { label: 'maxFrames', detail: 'Maximum frames to capture', insertText: 'maxFrames: ${1:150}', type: 'number' },
    { label: 'width', detail: 'Resize width', insertText: 'width: ${1:800}', type: 'number' },
  ],
  stopGifCapture: [
    { label: 'output', detail: 'Output GIF path (required)', insertText: 'output: ${1:"output.gif"}', type: 'string' },
    { label: 'delay', detail: 'Frame delay', insertText: 'delay: ${1:200}', type: 'number' },
    { label: 'quality', detail: 'GIF quality', insertText: 'quality: ${1|low,medium,high|}', type: 'string' },
    { label: 'loopCount', detail: 'Loop count', insertText: 'loopCount: ${1:0}', type: 'number' },
  ],
  exportReport: [
    { label: 'path', detail: 'Report file path (required)', insertText: 'path: ${1:"report.json"}', type: 'string' },
    { label: 'format', detail: 'Report format', insertText: 'format: ${1|json,xml,html|}', type: 'string' },
  ],
};

export const lumiYamlExtension: EditorExtension = {
  id: 'lumi-yaml',
  name: 'Lumi Tester YAML Support',
  languageId: 'yaml',
  fileExtensions: ['yaml', 'yml'],
  activate: (_registry: any) => {
    console.log('Lumi Tester YAML extension activated');
  },
  highlighter: null,
  lineDecorations: (_lineIndex, lineContent) => {
    const match = lineContent.match(/^\s*-\s*(\w+):\s*.*$/);
    if (match) {
      return {
        type: 'button',
        content: null,
        tooltip: `Run: ${match[1].trim()}`,
      };
    }
    return null;
  },
};

// Register YAML extension
extensionRegistry.register(lumiYamlExtension);
