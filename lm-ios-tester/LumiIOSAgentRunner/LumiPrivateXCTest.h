// Minimal, hand-written declarations for the small slice of XCTest's private API this
// agent needs for text input - not copied from WDA's auto-generated umbrella headers
// (which pull in ~15 unrelated transitive protocol files). These are declarations only,
// not implementations: the real symbols already exist in XCTest.framework on-device
// (this is exactly the same category of technique WebDriverAgent's own PrivateHeaders
// use - just trimmed to only what's actually called here).
//
// Signatures cross-checked against WebDriverAgent's PrivateHeaders/XCTest/{XCPointerEventPath,XCSynthesizedEventRecord,XCUIDevice,XCTRunnerDaemonSession}.h.
// The location-simulation slice (XCTRunnerDaemonSession) is the same private daemon RPC
// WebDriverAgent's own `fb_setSimulatedLocation:`/`FBXCTestDaemonsProxy` route through
// (WebDriverAgentLib/Categories/XCUIDevice+FBHelpers.m, WebDriverAgentLib/Utilities/
// FBXCTestDaemonsProxy.m) - confirmed to require iOS 16.4+ and a real device that
// reports `supportsLocationSimulation`, both true on this project's target device.

#import <XCTest/XCTest.h>
@import CoreLocation;

NS_ASSUME_NONNULL_BEGIN

@interface XCPointerEventPath : NSObject
- (instancetype)initForTextInput;
- (instancetype)initForTouchAtPoint:(CGPoint)point offset:(double)offset;
- (void)typeText:(NSString *)text atOffset:(double)offset typingSpeed:(unsigned long long)speed shouldRedact:(BOOL)redact;
- (void)liftUpAtOffset:(double)offset;
- (void)moveToPoint:(CGPoint)point atOffset:(double)offset;
@end

@interface XCSynthesizedEventRecord : NSObject
- (instancetype)initWithName:(NSString *)name;
- (void)addPointerEventPath:(XCPointerEventPath *)path;
@end

@protocol LumiEventSynthesizing <NSObject>
- (void)synthesizeEvent:(XCSynthesizedEventRecord *)event completion:(void (^)(BOOL success, NSError * _Nullable error))completion;
@end

@interface XCUIDevice (LumiPrivate)
- (id<LumiEventSynthesizing>)eventSynthesizer;
@end

@interface XCTRunnerDaemonSession : NSObject
+ (instancetype)sharedSession;
@property (readonly) BOOL supportsLocationSimulation;
- (void)setSimulatedLocation:(CLLocation *)location completion:(void (^)(BOOL, NSError * _Nullable))completion;
- (void)getSimulatedLocationWithReply:(void (^)(CLLocation * _Nullable, NSError * _Nullable))reply;
- (void)clearSimulatedLocationWithReply:(void (^)(BOOL, NSError * _Nullable))reply;
@end

NS_ASSUME_NONNULL_END
