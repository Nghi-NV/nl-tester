#import "LumiSocketServer.h"
#import "LumiCommandHandler.h"

#include <sys/socket.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <unistd.h>
#include <arpa/inet.h>

@interface LumiSocketServer ()
@property (nonatomic) uint16_t port;
@property (nonatomic) int listenFd;
@property (atomic) BOOL running;
@end

@implementation LumiSocketServer

- (instancetype)initWithPort:(uint16_t)port
{
  self = [super init];
  if (self) {
    _port = port;
    _listenFd = -1;
    _running = NO;
  }
  return self;
}

- (void)start
{
  self.running = YES;
  [NSThread detachNewThreadWithBlock:^{
    [self acceptLoop];
  }];
}

- (void)stop
{
  self.running = NO;
  if (self.listenFd >= 0) {
    close(self.listenFd);
    self.listenFd = -1;
  }
}

- (void)acceptLoop
{
  int fd = socket(AF_INET, SOCK_STREAM, 0);
  if (fd < 0) {
    NSLog(@"[LumiAgent] socket() failed: %d", errno);
    return;
  }
  self.listenFd = fd;

  int yes = 1;
  setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &yes, sizeof(yes));

  struct sockaddr_in addr;
  memset(&addr, 0, sizeof(addr));
  addr.sin_family = AF_INET;
  addr.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
  addr.sin_port = htons(self.port);

  if (bind(fd, (struct sockaddr *)&addr, sizeof(addr)) != 0) {
    NSLog(@"[LumiAgent] bind() on port %d failed: %d", self.port, errno);
    close(fd);
    return;
  }
  if (listen(fd, 4) != 0) {
    NSLog(@"[LumiAgent] listen() failed: %d", errno);
    close(fd);
    return;
  }
  NSLog(@"[LumiAgent] Listening on 127.0.0.1:%d", self.port);

  while (self.running) {
    struct sockaddr_in clientAddr;
    socklen_t clientLen = sizeof(clientAddr);
    int clientFd = accept(fd, (struct sockaddr *)&clientAddr, &clientLen);
    if (clientFd < 0) {
      if (!self.running) break;
      continue;
    }
    int one = 1;
    setsockopt(clientFd, IPPROTO_TCP, TCP_NODELAY, &one, sizeof(one));
    NSLog(@"[LumiAgent] Client connected");
    [self handleClient:clientFd];
    NSLog(@"[LumiAgent] Client disconnected");
  }
}

- (void)handleClient:(int)clientFd
{
  NSMutableData *buffer = [NSMutableData data];
  uint8_t chunk[4096];

  while (self.running) {
    ssize_t n = read(clientFd, chunk, sizeof(chunk));
    if (n <= 0) {
      break;
    }
    [buffer appendBytes:chunk length:(NSUInteger)n];

    NSRange newline;
    while ((newline = [buffer rangeOfData:[NSData dataWithBytes:"\n" length:1]
                                   options:0
                                     range:NSMakeRange(0, buffer.length)]).location != NSNotFound) {
      NSData *lineData = [buffer subdataWithRange:NSMakeRange(0, newline.location)];
      [buffer replaceBytesInRange:NSMakeRange(0, newline.location + 1) withBytes:NULL length:0];

      if (lineData.length == 0) {
        continue;
      }
      NSString *line = [[NSString alloc] initWithData:lineData encoding:NSUTF8StringEncoding];
      NSString *response = [LumiCommandHandler handleCommandLine:line];
      NSData *out = [[response stringByAppendingString:@"\n"] dataUsingEncoding:NSUTF8StringEncoding];
      const uint8_t *bytes = out.bytes;
      NSUInteger remaining = out.length;
      while (remaining > 0) {
        ssize_t written = write(clientFd, bytes, remaining);
        if (written <= 0) {
          break;
        }
        bytes += written;
        remaining -= (NSUInteger)written;
      }
    }
  }
  close(clientFd);
}

@end
