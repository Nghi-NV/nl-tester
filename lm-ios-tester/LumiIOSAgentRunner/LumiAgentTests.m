#import <XCTest/XCTest.h>
#import <objc/runtime.h>
#import "LumiSocketServer.h"

// Port 8110, deliberately distinct from WebDriverAgent's 8100 so both can run
// side-by-side on-device during the Stage A rollout in the parent plan.
static const uint16_t kLumiAgentPort = 8110;

@interface LumiAgentTests : XCTestCase
@property (atomic) BOOL keepAlive;
@end

@implementation LumiAgentTests

// XCTest normally runs its automatic UI-interruption monitor (checks for system alerts/
// permission dialogs and auto-dismisses them) around every synthesized UI action -
// WebDriverAgent disables this per-action overhead once, process-wide, at startup via
// method swizzling: WebDriverAgentLib/Categories/XCUIApplication+FBUIInterruptions.m
// replaces `-[XCUIApplication doesNotHandleUIInterruptions]` (a private XCTest method
// that already exists in the runtime, just not declared in any public header - confirmed
// via `class_getInstanceMethod` unconditionally succeeding in WDA's own swizzle helper,
// FBReflectionUtils.m) to always return YES. Measured (see LumiTiming logs) that without
// this, `synthesizeEvent:completion:`'s completion block genuinely does not fire for
// ~285ms after being issued on this real device/app - suspiciously close to a
// once-per-interaction system-alert-check round trip - and WDA calls this exact swizzle
// in its own `+setUp` (WebDriverAgentRunner/UITestingUITests.m).
+ (void)setUp
{
  [super setUp];
  Method original = class_getInstanceMethod([XCUIApplication class], @selector(doesNotHandleUIInterruptions));
  if (original) {
    IMP alwaysYes = imp_implementationWithBlock(^BOOL(id self_) { return YES; });
    method_setImplementation(original, alwaysYes);
    NSLog(@"[LumiAgent] Swizzled -[XCUIApplication doesNotHandleUIInterruptions] to always YES");
  } else {
    NSLog(@"[LumiAgent] WARNING: -doesNotHandleUIInterruptions not found on XCUIApplication - interruption monitor NOT disabled");
  }
}

// Never-ending test used to keep the agent process alive - same idiom WebDriverAgentRunner's
// UITestingUITests.testRunner uses via FBWebServer.startServing (spin the main run loop on
// this test's thread while a background thread accepts socket connections).
- (void)testRunner
{
  LumiSocketServer *server = [[LumiSocketServer alloc] initWithPort:kLumiAgentPort];
  [server start];

  self.keepAlive = YES;
  NSRunLoop *runLoop = [NSRunLoop mainRunLoop];
  while (self.keepAlive &&
         [runLoop runMode:NSDefaultRunLoopMode beforeDate:[NSDate distantFuture]]) {
  }
}

@end
