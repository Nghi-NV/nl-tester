#import <Foundation/Foundation.h>

NS_ASSUME_NONNULL_BEGIN

// One JSON object per line in, one JSON object per line out, over a persistent
// connection - mirrors lm-android-tester's CommandServer/CommandHandler protocol shape.
@interface LumiSocketServer : NSObject

- (instancetype)initWithPort:(uint16_t)port;
- (void)start;
- (void)stop;

@end

NS_ASSUME_NONNULL_END
