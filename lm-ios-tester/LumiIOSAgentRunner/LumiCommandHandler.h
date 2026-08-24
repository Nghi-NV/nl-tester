#import <Foundation/Foundation.h>

NS_ASSUME_NONNULL_BEGIN

@interface LumiCommandHandler : NSObject
+ (NSString *)handleCommandLine:(NSString *)line;
@end

NS_ASSUME_NONNULL_END
