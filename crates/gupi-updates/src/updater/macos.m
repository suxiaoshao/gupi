#import <Cocoa/Cocoa.h>

// A dynamically loaded Sparkle 2 bridge. These selectors are from its public API;
// the application remains runnable from source without a framework installed.
@interface SPUUpdater : NSObject
@property BOOL automaticallyChecksForUpdates;
@property BOOL automaticallyDownloadsUpdates;
@property BOOL sendsSystemProfile;
@property NSURL *feedURL;
@property(readonly) BOOL canCheckForUpdates;
- (BOOL)startUpdater:(NSError **)error;
- (void)checkForUpdates;
@end
@interface SPUStandardUpdaterController : NSObject
- (instancetype)initWithStartingUpdater:(BOOL)start updaterDelegate:(id)delegate userDriverDelegate:(id)driver;
@property(readonly) SPUUpdater *updater;
@end

static void (*eventCallback)(int);
static SPUStandardUpdaterController *controller;
static NSBundle *framework;
static void (^resumeInstallation)(void);

@interface GupiUpdaterDelegate : NSObject
@end
@implementation GupiUpdaterDelegate
- (void)updater:(id)updater userDidMakeChoice:(NSInteger)choice forUpdate:(id)item state:(id)state {
    (void)updater; (void)item; (void)state;
    // SPUUserUpdateChoiceSkip is the first value in Sparkle's public enum.
    if (choice == 0) eventCallback(4);
}
- (BOOL)updater:(id)updater shouldPostponeRelaunchForUpdate:(id)item untilInvokingBlock:(void (^)(void))handler {
    (void)updater; (void)item;
    resumeInstallation = [handler copy];
    eventCallback(1);
    return YES;
}
- (void)updater:(id)updater didFinishUpdateCycleForUpdateCheck:(NSInteger)check error:(NSError *)error {
    (void)updater; (void)check;
    eventCallback(error ? 3 : 2);
}
@end
static GupiUpdaterDelegate *delegate;

int gupi_updater_available(void) {
    NSBundle *app = [NSBundle mainBundle];
    NSString *key = [app objectForInfoDictionaryKey:@"SUPublicEDKey"];
    NSString *path = [[app privateFrameworksPath] stringByAppendingPathComponent:@"Sparkle.framework"];
    return key.length > 0 && [[NSFileManager defaultManager] fileExistsAtPath:path];
}

int gupi_updater_init(void (*callback)(int)) {
    if (!gupi_updater_available()) return 0;
    eventCallback = callback;
    NSString *path = [[[NSBundle mainBundle] privateFrameworksPath] stringByAppendingPathComponent:@"Sparkle.framework"];
    framework = [NSBundle bundleWithPath:path];
    NSError *error = nil;
    if (![framework loadAndReturnError:&error]) { NSLog(@"Gupi updater: %@", error); return 0; }
    Class cls = NSClassFromString(@"SPUStandardUpdaterController");
    if (!cls) return 0;
    delegate = [GupiUpdaterDelegate new];
    controller = [[cls alloc] initWithStartingUpdater:NO updaterDelegate:delegate userDriverDelegate:nil];
    SPUUpdater *updater = controller.updater;
    updater.automaticallyChecksForUpdates = NO;
    updater.automaticallyDownloadsUpdates = NO;
    updater.sendsSystemProfile = NO;
    if (![updater startUpdater:&error]) { NSLog(@"Gupi updater: %@", error); return 0; }
    return 1;
}

int gupi_updater_install(const char *feed) {
    if (!controller.updater.canCheckForUpdates) return 0;
    NSURL *url = [NSURL URLWithString:[NSString stringWithUTF8String:feed]];
    // Native window activation may synchronously call into GPUI. Leave its App
    // transaction before asking Sparkle to show its window.
    dispatch_async(dispatch_get_main_queue(), ^{
        controller.updater.feedURL = url;
        [controller.updater checkForUpdates];
    });
    return 1;
}

void gupi_updater_resume(void) {
    // Called on GPUI's main thread only after all managed shutdown work completes.
    void (^handler)(void) = resumeInstallation;
    resumeInstallation = nil;
    if (handler) dispatch_async(dispatch_get_main_queue(), handler);
}

void gupi_updater_show(void) {
    dispatch_async(dispatch_get_main_queue(), ^{
        // Sparkle brings an existing download/install session back into focus.
        [controller.updater checkForUpdates];
    });
}
