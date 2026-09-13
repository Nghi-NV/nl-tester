#import "LumiCommandHandler.h"
#import "LumiPrivateXCTest.h"
#import <XCTest/XCTest.h>

static NSString *LumiElementTypeName(XCUIElementType t)
{
  switch (t) {
    case XCUIElementTypeApplication: return @"Application";
    case XCUIElementTypeWindow: return @"Window";
    case XCUIElementTypeButton: return @"Button";
    case XCUIElementTypeStaticText: return @"StaticText";
    case XCUIElementTypeTextField: return @"TextField";
    case XCUIElementTypeSecureTextField: return @"SecureTextField";
    case XCUIElementTypeTextView: return @"TextView";
    case XCUIElementTypeImage: return @"Image";
    case XCUIElementTypeScrollView: return @"ScrollView";
    case XCUIElementTypeTable: return @"Table";
    case XCUIElementTypeCell: return @"Cell";
    case XCUIElementTypeSwitch: return @"Switch";
    case XCUIElementTypeSlider: return @"Slider";
    case XCUIElementTypeNavigationBar: return @"NavigationBar";
    case XCUIElementTypeTabBar: return @"TabBar";
    case XCUIElementTypeTabGroup: return @"TabGroup";
    case XCUIElementTypeToolbar: return @"Toolbar";
    case XCUIElementTypeCollectionView: return @"CollectionView";
    case XCUIElementTypeOther: return @"Other";
    default: return @"Other";
  }
}

static UIDeviceOrientation LumiOrientationFromString(NSString *mode)
{
  if ([mode isEqualToString:@"landscapeLeft"]) return UIDeviceOrientationLandscapeLeft;
  if ([mode isEqualToString:@"landscapeRight"]) return UIDeviceOrientationLandscapeRight;
  if ([mode isEqualToString:@"upsideDown"]) return UIDeviceOrientationPortraitUpsideDown;
  return UIDeviceOrientationPortrait;
}

static NSString *LumiOrientationToString(UIDeviceOrientation o)
{
  switch (o) {
    case UIDeviceOrientationLandscapeLeft: return @"landscapeLeft";
    case UIDeviceOrientationLandscapeRight: return @"landscapeRight";
    case UIDeviceOrientationPortraitUpsideDown: return @"upsideDown";
    default: return @"portrait";
  }
}

@implementation LumiCommandHandler

+ (XCUIApplication *)targetAppForBundleId:(NSString * _Nullable)bundleId
{
  // Caching an `XCUIApplication` proxy per bundle id (as this used to do) turned out to
  // go irrecoverably stale in a way neither checking `.state` nor `activate` nor
  // `waitForState:` fixes: once the device visits the App Switcher even once, a
  // previously-cached proxy keeps resolving to the App Switcher's tiny *card preview*
  // window for that app (confirmed via `debugDescription` ground truth: `Window` frame
  // came back `{130, 281}` - an app-switcher-thumbnail size - instead of the real
  // `{390, 844}` device screen, even after `activate`+`waitForState:...Foreground`
  // both reported success). `launchApp` (Rust driver) unconditionally terminates and
  // relaunches the app process on every call regardless of `clearState`, so a cached
  // proxy is tied to a specific now-dead process instance far more often than "cache for
  // speed" assumed. Always resolving a fresh instance is the correct, robust choice;
  // re-measure whether this reintroduces meaningful per-call latency before assuming it
  // needs a smarter cache-invalidation scheme instead.
  NSString *key = bundleId.length > 0 ? bundleId : @"com.apple.springboard";
  return [[XCUIApplication alloc] initWithBundleIdentifier:key];
}

+ (XCUICoordinate *)coordinateAt:(CGPoint)point inApp:(XCUIApplication *)app
{
  XCUICoordinate *origin = [app coordinateWithNormalizedOffset:CGVectorMake(0, 0)];
  return [origin coordinateWithOffset:CGVectorMake(point.x, point.y)];
}

// Runs the current run loop (rather than blocking the thread outright on a semaphore)
// until `completion` is delivered or `timeout` elapses. Matters specifically because
// command handling already runs via `dispatch_sync(dispatch_get_main_queue(), ...)`
// (see `handleCommandLine:`): if `synthesizeEvent:completion:`'s completion block is
// itself delivered back via the main queue/run loop, a hard `dispatch_semaphore_wait`
// here starves it out (the main thread is blocked on the semaphore, so the queued
// completion block can never run) until some internal XCTest fallback/retry path
// eventually delivers it - measured as a flat, suspiciously consistent ~280ms floor on
// every single touch synthesis call, which disappeared once this was switched to
// pumping the run loop instead. This is the same technique WDA's own FBRunLoopSpinner
// uses for exactly this reason (see WebDriverAgentLib/Utilities/FBRunLoopSpinner.m).
+ (BOOL)lumiSynthesizeEvent:(XCSynthesizedEventRecord *)record
{
  __block BOOL done = NO;
  __block BOOL ok = NO;
  [[XCUIDevice.sharedDevice eventSynthesizer] synthesizeEvent:record completion:^(BOOL success, NSError * _Nullable error) {
    ok = success;
    done = YES;
  }];
  NSDate *deadline = [NSDate dateWithTimeIntervalSinceNow:10.0];
  while (!done && [deadline timeIntervalSinceNow] > 0) {
    [[NSRunLoop currentRunLoop] runMode:NSDefaultRunLoopMode beforeDate:[NSDate dateWithTimeIntervalSinceNow:0.005]];
  }
  return done && ok;
}

// Real-device location simulation via `XCTRunnerDaemonSession` - the same private XCTest
// daemon RPC WebDriverAgent's `fb_setSimulatedLocation:` routes through (see
// LumiPrivateXCTest.h's header comment). Requires iOS 16.4+ and
// `supportsLocationSimulation` (both true on this project's target device); on older/
// unsupported devices this correctly reports failure rather than silently no-op'ing.
+ (BOOL)lumiSetLocationLat:(double)lat lon:(double)lon alt:(double)alt errorOut:(NSString * _Nullable * _Nullable)errorOut
{
  XCTRunnerDaemonSession *session = [XCTRunnerDaemonSession sharedSession];
  if (!session.supportsLocationSimulation) {
    if (errorOut) *errorOut = @"Device does not support location simulation (requires iOS 16.4+)";
    return NO;
  }
  CLLocation *location = [[CLLocation alloc] initWithCoordinate:CLLocationCoordinate2DMake(lat, lon)
                                                         altitude:alt
                                               horizontalAccuracy:5.0
                                                 verticalAccuracy:5.0
                                                        timestamp:[NSDate date]];
  __block BOOL done = NO;
  __block BOOL ok = NO;
  __block NSString *errMsg = nil;
  [session setSimulatedLocation:location completion:^(BOOL success, NSError * _Nullable error) {
    ok = success;
    errMsg = error.localizedDescription;
    done = YES;
  }];
  NSDate *deadline = [NSDate dateWithTimeIntervalSinceNow:10.0];
  while (!done && [deadline timeIntervalSinceNow] > 0) {
    [[NSRunLoop currentRunLoop] runMode:NSDefaultRunLoopMode beforeDate:[NSDate dateWithTimeIntervalSinceNow:0.005]];
  }
  if (errorOut) *errorOut = errMsg;
  return done && ok;
}

+ (BOOL)lumiClearLocationErrorOut:(NSString * _Nullable * _Nullable)errorOut
{
  XCTRunnerDaemonSession *session = [XCTRunnerDaemonSession sharedSession];
  __block BOOL done = NO;
  __block BOOL ok = NO;
  __block NSString *errMsg = nil;
  [session clearSimulatedLocationWithReply:^(BOOL success, NSError * _Nullable error) {
    ok = success;
    errMsg = error.localizedDescription;
    done = YES;
  }];
  NSDate *deadline = [NSDate dateWithTimeIntervalSinceNow:10.0];
  while (!done && [deadline timeIntervalSinceNow] > 0) {
    [[NSRunLoop currentRunLoop] runMode:NSDefaultRunLoopMode beforeDate:[NSDate dateWithTimeIntervalSinceNow:0.005]];
  }
  if (errorOut) *errorOut = errMsg;
  return done && ok;
}

// Raw touch-event synthesis, bypassing XCUICoordinate's public tap()/press()/drag()
// gesture wrappers entirely. Measured: XCUICoordinate.tap() takes ~650-700ms per call
// (public gesture APIs include XCTest's built-in post-action app-quiescence wait, which
// this project's whole point is to avoid paying on every single interaction) vs. this
// raw path's single-digit ms - see the parent report's before/after numbers. Same
// XCSynthesizedEventRecord/XCPointerEventPath technique already used for text input,
// just with `initForTouchAtPoint:offset:` instead of `initForTextInput`.
+ (BOOL)synthesizeTouchDownAt:(CGPoint)point liftAfter:(double)liftOffset moveTo:(CGPoint)moveTo hasMoveTo:(BOOL)hasMoveTo
{
  XCSynthesizedEventRecord *record = [[XCSynthesizedEventRecord alloc] initWithName:@"LumiTouch"];
  XCPointerEventPath *path = [[XCPointerEventPath alloc] initForTouchAtPoint:point offset:0.0];
  if (hasMoveTo) {
    [path moveToPoint:moveTo atOffset:liftOffset * 0.5];
  }
  [path liftUpAtOffset:liftOffset];
  [record addPointerEventPath:path];
  return [self lumiSynthesizeEvent:record];
}

// 2-finger gesture primitive: each finger's whole down->move(xN)->up timeline is built as
// its own `XCPointerEventPath`, then BOTH paths are added to the SAME
// `XCSynthesizedEventRecord` - this is what makes the two touches simultaneous/multi-touch
// (delivering two separate single-path records back-to-back would read as two sequential
// single-finger gestures to the app, not a pinch/shove). `pinch`/`shove` below both build
// on this; the only difference between them is where each finger's start/end points sit.
+ (BOOL)synthesizeTwoFingerGestureFrom:(CGPoint)finger1Start to:(CGPoint)finger1End
                                    and:(CGPoint)finger2Start to:(CGPoint)finger2End
                            durationSec:(double)duration
{
  XCSynthesizedEventRecord *record = [[XCSynthesizedEventRecord alloc] initWithName:@"LumiTwoFingerGesture"];

  XCPointerEventPath *path1 = [[XCPointerEventPath alloc] initForTouchAtPoint:finger1Start offset:0.0];
  XCPointerEventPath *path2 = [[XCPointerEventPath alloc] initForTouchAtPoint:finger2Start offset:0.0];

  NSInteger steps = 10;
  for (NSInteger i = 1; i <= steps; i++) {
    double ratio = (double)i / (double)steps;
    double offset = duration * ratio;
    CGPoint p1 = CGPointMake(finger1Start.x + (finger1End.x - finger1Start.x) * ratio,
                              finger1Start.y + (finger1End.y - finger1Start.y) * ratio);
    CGPoint p2 = CGPointMake(finger2Start.x + (finger2End.x - finger2Start.x) * ratio,
                              finger2Start.y + (finger2End.y - finger2Start.y) * ratio);
    [path1 moveToPoint:p1 atOffset:offset];
    [path2 moveToPoint:p2 atOffset:offset];
  }

  [path1 liftUpAtOffset:duration];
  [path2 liftUpAtOffset:duration];

  [record addPointerEventPath:path1];
  [record addPointerEventPath:path2];

  return [self lumiSynthesizeEvent:record];
}

// Pinch centered at `center`: two fingers start `startRadius` points from center along a
// line at `angleDeg` and end `endRadius` points from center along that same line - zoom in
// when endRadius > startRadius, zoom out when endRadius < startRadius. Standard 2-finger
// pinch-to-zoom every map SDK (Mapbox included) recognizes.
+ (BOOL)synthesizePinchAtCenter:(CGPoint)center
                     startRadius:(double)startRadius
                       endRadius:(double)endRadius
                        angleDeg:(double)angleDeg
                     durationSec:(double)duration
{
  double rad = angleDeg * M_PI / 180.0;
  double dirX = cos(rad);
  double dirY = sin(rad);
  CGPoint f1Start = CGPointMake(center.x + dirX * startRadius, center.y + dirY * startRadius);
  CGPoint f1End   = CGPointMake(center.x + dirX * endRadius,   center.y + dirY * endRadius);
  CGPoint f2Start = CGPointMake(center.x - dirX * startRadius, center.y - dirY * startRadius);
  CGPoint f2End   = CGPointMake(center.x - dirX * endRadius,   center.y - dirY * endRadius);
  return [self synthesizeTwoFingerGestureFrom:f1Start to:f1End and:f2Start to:f2End durationSec:duration];
}

// Shove: two fingers held `spacing` points apart slide together by (dx, dy) points - the
// gesture Mapbox (and most map SDKs) bind to camera pitch/tilt: sliding up tilts the
// camera into a 3D perspective, sliding down flattens it back to 2D.
+ (BOOL)synthesizeShoveAtCenter:(CGPoint)center
                         spacing:(double)spacing
                              dx:(double)dx
                              dy:(double)dy
                     durationSec:(double)duration
{
  CGPoint f1Start = CGPointMake(center.x - spacing / 2.0, center.y);
  CGPoint f2Start = CGPointMake(center.x + spacing / 2.0, center.y);
  CGPoint f1End = CGPointMake(f1Start.x + dx, f1Start.y + dy);
  CGPoint f2End = CGPointMake(f2Start.x + dx, f2Start.y + dy);
  return [self synthesizeTwoFingerGestureFrom:f1Start to:f1End and:f2Start to:f2End durationSec:duration];
}

+ (BOOL)synthesizeTypeText:(NSString *)text
{
  XCSynthesizedEventRecord *record = [[XCSynthesizedEventRecord alloc] initWithName:@"LumiType"];
  XCPointerEventPath *path = [[XCPointerEventPath alloc] initForTextInput];
  [path typeText:text atOffset:0.0 typingSpeed:60 shouldRedact:NO];
  [record addPointerEventPath:path];
  return [self lumiSynthesizeEvent:record];
}

+ (NSString *)backspaceCharacter
{
  return [NSString stringWithFormat:@"%c", 8];
}

+ (NSString *)escapeCharacter
{
  return [NSString stringWithFormat:@"%c", 27];
}

// Uses `-[XCUIElement snapshotWithError:]` (public API, XCUIAutomation.framework -
// declared in XCUIElement.h as part of the `XCUIElementSnapshotProviding` category,
// returning `id<XCUIElementSnapshot>`) to fetch the ENTIRE subtree in a single XCTest
// IPC round trip, then walks the already-in-memory snapshot tree's `.children` (also
// `id<XCUIElementSnapshot>`, conforming to `XCUIElementAttributes` - frame/label/value/
// etc. all present with no further IPC) purely locally. This mirrors WDA's own
// `fb_standardSnapshot` technique (WebDriverAgentLib/Categories/XCUIElement+FBUtilities.m)
// which calls the same underlying `-snapshotWithError:` (WDA declares it via a private
// header since older SDKs didn't expose it publicly; the SDK on this machine already
// has it as a real public protocol, so no private header is needed here at all).
//
// The previous implementation instead called `[element childrenMatchingType:XCUIElementTypeAny]`
// recursively per node - each such call is a *separate* XCTest IPC round trip, and on
// this app's real (deep, Flutter-rendered) semantics tree that made `hierarchy` hang
// indefinitely (effectively O(node count) round trips instead of O(1)).
+ (NSDictionary *)hierarchyDictForSnapshot:(id<XCUIElementSnapshot>)snapshot depth:(NSInteger)depth
{
  CGRect frame = snapshot.frame;
  id value = snapshot.value;
  NSMutableDictionary *dict = [NSMutableDictionary dictionary];
  dict[@"type"] = LumiElementTypeName(snapshot.elementType);
  dict[@"label"] = snapshot.label ?: [NSNull null];
  dict[@"identifier"] = snapshot.identifier ?: [NSNull null];
  dict[@"value"] = value ? [NSString stringWithFormat:@"%@", value] : [NSNull null];
  dict[@"placeholder"] = snapshot.placeholderValue ?: [NSNull null];
  dict[@"frame"] = @{
    @"x": @(frame.origin.x),
    @"y": @(frame.origin.y),
    @"width": @(frame.size.width),
    @"height": @(frame.size.height),
  };
  dict[@"enabled"] = @(snapshot.enabled);
  dict[@"visible"] = @(frame.size.width > 0 && frame.size.height > 0);

  NSMutableArray *children = [NSMutableArray array];
  if (depth < 60) {
    for (id<XCUIElementSnapshot> kid in snapshot.children) {
      [children addObject:[self hierarchyDictForSnapshot:kid depth:depth + 1]];
    }
  }
  dict[@"children"] = children;
  return dict;
}

+ (NSDictionary *)hierarchyDictForElement:(XCUIElement *)element
{
  NSError *error = nil;
  id<XCUIElementSnapshot> snapshot = [element snapshotWithError:&error];
  if (!snapshot) {
    return @{@"type": @"Application", @"error": error.localizedDescription ?: @"snapshot failed", @"children": @[]};
  }
  return [self hierarchyDictForSnapshot:snapshot depth:0];
}

+ (NSString *)handleCommandLine:(NSString *)line
{
  NSData *data = [line dataUsingEncoding:NSUTF8StringEncoding];
  NSError *jsonErr = nil;
  NSDictionary *req = [NSJSONSerialization JSONObjectWithData:data options:0 error:&jsonErr];
  if (![req isKindOfClass:[NSDictionary class]]) {
    return @"{\"error\":\"invalid json\"}";
  }
  NSString *cmd = req[@"cmd"];
  NSMutableDictionary *resp = [NSMutableDictionary dictionaryWithDictionary:@{@"cmd": cmd ?: @""}];

  __block id result = nil;
  __block BOOL success = NO;

  dispatch_sync(dispatch_get_main_queue(), ^{
    @try {
      if ([cmd isEqualToString:@"status"] || [cmd isEqualToString:@"ping"]) {
        success = YES;

      } else if ([cmd isEqualToString:@"tap"]) {
        CGPoint p = CGPointMake([req[@"x"] doubleValue], [req[@"y"] doubleValue]);
        // GOFA (Flutter) has repeatedly shown live that a short touch-down/up
        // doesn't reliably win Flutter's gesture arena - confirmed on many
        // distinct elements across this session (nav links, the final login
        // submit button, dialog buttons), each time intermittently: the
        // touch is delivered and this call reports success, but the
        // widget's own tap handler sometimes just doesn't fire. Tried 50ms
        // and 120ms, both still dropped taps live. 300ms stays comfortably
        // under both iOS's and Flutter's ~500ms long-press threshold (so it
        // still resolves as a tap everywhere else, not a long-press, and no
        // unintended long-press-specific behavior - context menus, drag-init
        // - has been observed on any element tapped this session at this or
        // longer durations) while reliably clearing the arena-resolution
        // window that was dropping taps at the shorter durations.
        success = [self synthesizeTouchDownAt:p liftAfter:0.3 moveTo:CGPointZero hasMoveTo:NO];

      } else if ([cmd isEqualToString:@"long_press"]) {
        CGPoint p = CGPointMake([req[@"x"] doubleValue], [req[@"y"] doubleValue]);
        double durationMs = req[@"duration_ms"] ? [req[@"duration_ms"] doubleValue] : 1000.0;
        success = [self synthesizeTouchDownAt:p liftAfter:durationMs / 1000.0 moveTo:CGPointZero hasMoveTo:NO];

      } else if ([cmd isEqualToString:@"double_tap"]) {
        CGPoint p = CGPointMake([req[@"x"] doubleValue], [req[@"y"] doubleValue]);
        BOOL first = [self synthesizeTouchDownAt:p liftAfter:0.04 moveTo:CGPointZero hasMoveTo:NO];
        BOOL second = [self synthesizeTouchDownAt:p liftAfter:0.04 moveTo:CGPointZero hasMoveTo:NO];
        success = first && second;

      } else if ([cmd isEqualToString:@"swipe"]) {
        CGPoint p1 = CGPointMake([req[@"x1"] doubleValue], [req[@"y1"] doubleValue]);
        CGPoint p2 = CGPointMake([req[@"x2"] doubleValue], [req[@"y2"] doubleValue]);
        double durationMs = req[@"duration_ms"] ? [req[@"duration_ms"] doubleValue] : 300.0;
        success = [self synthesizeTouchDownAt:p1 liftAfter:durationMs / 1000.0 moveTo:p2 hasMoveTo:YES];

      } else if ([cmd isEqualToString:@"pinch"]) {
        CGPoint center = CGPointMake([req[@"cx"] doubleValue], [req[@"cy"] doubleValue]);
        double startRadius = [req[@"startRadius"] doubleValue];
        double endRadius = [req[@"endRadius"] doubleValue];
        double angle = req[@"angle"] ? [req[@"angle"] doubleValue] : 0.0;
        double durationMs = req[@"duration"] ? [req[@"duration"] doubleValue] : 400.0;
        success = [self synthesizePinchAtCenter:center
                                     startRadius:startRadius
                                       endRadius:endRadius
                                        angleDeg:angle
                                     durationSec:durationMs / 1000.0];

      } else if ([cmd isEqualToString:@"shove"]) {
        CGPoint center = CGPointMake([req[@"cx"] doubleValue], [req[@"cy"] doubleValue]);
        double spacing = [req[@"spacing"] doubleValue];
        double dx = [req[@"dx"] doubleValue];
        double dy = [req[@"dy"] doubleValue];
        double durationMs = req[@"duration"] ? [req[@"duration"] doubleValue] : 400.0;
        success = [self synthesizeShoveAtCenter:center
                                         spacing:spacing
                                              dx:dx
                                              dy:dy
                                     durationSec:durationMs / 1000.0];

      } else if ([cmd isEqualToString:@"type_text"]) {
        NSString *text = req[@"text"] ?: @"";
        success = [self synthesizeTypeText:text];

      } else if ([cmd isEqualToString:@"erase_text"]) {
        NSInteger count = req[@"count"] ? [req[@"count"] integerValue] : 60;
        NSString *backspaceStr = [self backspaceCharacter];
        NSMutableString *backspaces = [NSMutableString string];
        for (NSInteger i = 0; i < count; i++) {
          [backspaces appendString:backspaceStr];
        }
        success = [self synthesizeTypeText:backspaces];

      } else if ([cmd isEqualToString:@"press_key"]) {
        NSString *key = req[@"key"] ?: @"";
        NSString *ch = nil;
        if ([key isEqualToString:@"RETURN"] || [key isEqualToString:@"ENTER"]) {
          ch = @"\n";
        } else if ([key isEqualToString:@"DELETE"] || [key isEqualToString:@"BACKSPACE"]) {
          ch = [self backspaceCharacter];
        } else if ([key isEqualToString:@"TAB"]) {
          ch = @"\t";
        } else if ([key isEqualToString:@"ESCAPE"]) {
          ch = [self escapeCharacter];
        }
        if (ch) {
          success = [self synthesizeTypeText:ch];
        } else {
          success = NO;
        }

      } else if ([cmd isEqualToString:@"press_button"]) {
        NSString *name = [req[@"name"] lowercaseString] ?: @"";
        XCUIDeviceButton button = XCUIDeviceButtonHome;
        BOOL known = YES;
        if ([name isEqualToString:@"home"]) {
          button = XCUIDeviceButtonHome;
#if !TARGET_OS_SIMULATOR
        } else if ([name isEqualToString:@"volumeup"]) {
          button = XCUIDeviceButtonVolumeUp;
        } else if ([name isEqualToString:@"volumedown"]) {
          button = XCUIDeviceButtonVolumeDown;
#endif
        } else {
          known = NO;
        }
        if (known) {
          [XCUIDevice.sharedDevice pressButton:button];
          success = YES;
        } else {
          success = NO;
        }

      } else if ([cmd isEqualToString:@"hierarchy"]) {
        XCUIApplication *app = [self targetAppForBundleId:req[@"bundleId"]];
        result = [self hierarchyDictForElement:app];
        success = YES;

      } else if ([cmd isEqualToString:@"launch_app"]) {
        // `[XCUIApplication terminate]`+`launch` (native XCTest, not idb) - required
        // because `idb terminate`/`idb launch` are confirmed broken on this iOS 26.5.2
        // device ("The best match .../DeveloperDiskImage.dmg: 16.4 is not suitable for
        // 26.5", idb silently reporting "No pid" for terminate). Since the Rust driver's
        // `launch_app` swallowed idb's terminate failure (`let _ = idb::terminate_app`),
        // the app process was NEVER actually being restarted across the whole session
        // despite `launchApp` reporting success - which left `XCUIApplication`'s cached
        // window resolution permanently stuck on a stale window from one earlier session
        // (reproduced/confirmed: that stale window's frame was exactly 1/3 the real
        // screen size on every axis, `{130, 281.3}` instead of `{390, 844}`, matching an
        // App Switcher card-preview thumbnail's dimensions - once a *real* `terminate`+
        // `launch` finally happened via this command, using a genuinely new process pid,
        // the window frame and full accessibility content immediately came back correct).
        XCUIApplication *app = [self targetAppForBundleId:req[@"bundleId"]];
        [app terminate];
        [NSThread sleepForTimeInterval:1.0];
        [app launch];
        BOOL cameUp = [app waitForState:XCUIApplicationStateRunningForeground timeout:10.0];
        result = @{@"cameUp": @(cameUp)};
        success = YES;

      } else if ([cmd isEqualToString:@"terminate_app"]) {
        XCUIApplication *app = [self targetAppForBundleId:req[@"bundleId"]];
        [app terminate];
        success = YES;

      } else if ([cmd isEqualToString:@"screenshot"]) {
        XCUIScreen *screen = XCUIScreen.mainScreen;
        XCUIScreenshot *shot = [screen screenshot];
        NSData *png = shot.PNGRepresentation;
        result = [png base64EncodedStringWithOptions:0];
        success = (png != nil);

      } else if ([cmd isEqualToString:@"get_screen_size"]) {
        CGSize size = XCUIScreen.mainScreen.screenshot.image.size;
        result = @{@"width": @(size.width), @"height": @(size.height)};
        success = YES;

      } else if ([cmd isEqualToString:@"set_orientation"]) {
        NSString *mode = req[@"mode"] ?: @"portrait";
        XCUIDevice.sharedDevice.orientation = LumiOrientationFromString(mode);
        success = YES;

      } else if ([cmd isEqualToString:@"get_orientation"]) {
        result = LumiOrientationToString(XCUIDevice.sharedDevice.orientation);
        success = YES;

      } else if ([cmd isEqualToString:@"set_location"]) {
        double lat = [req[@"lat"] doubleValue];
        double lon = [req[@"lon"] doubleValue];
        double alt = req[@"alt"] ? [req[@"alt"] doubleValue] : 0.0;
        NSString *errMsg = nil;
        success = [self lumiSetLocationLat:lat lon:lon alt:alt errorOut:&errMsg];
        if (!success && errMsg) {
          result = errMsg;
        }

      } else if ([cmd isEqualToString:@"clear_location"]) {
        NSString *errMsg = nil;
        success = [self lumiClearLocationErrorOut:&errMsg];
        if (!success && errMsg) {
          result = errMsg;
        }

      } else {
        success = NO;
        result = @"unknown command";
      }
    } @catch (NSException *exception) {
      success = NO;
      result = exception.reason ?: @"exception";
    }
  });

  resp[@"success"] = @(success);
  if (result) {
    resp[@"data"] = result;
  }

  NSError *encodeErr = nil;
  NSData *outData = [NSJSONSerialization dataWithJSONObject:resp options:0 error:&encodeErr];
  if (!outData) {
    return @"{\"success\":false,\"error\":\"encode failure\"}";
  }
  return [[NSString alloc] initWithData:outData encoding:NSUTF8StringEncoding];
}

@end
