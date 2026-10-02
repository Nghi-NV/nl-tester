use std::path::PathBuf;

pub fn record(app: &tauri::AppHandle, path: PathBuf) {
    #[cfg(target_os = "macos")]
    {
        let result = app.run_on_main_thread(move || {
            use objc2::MainThreadMarker;
            use objc2_app_kit::NSDocumentController;
            use objc2_foundation::{NSString, NSURL};

            let Some(main_thread) = MainThreadMarker::new() else {
                return;
            };
            let controller = NSDocumentController::sharedDocumentController(main_thread);
            let path = NSString::from_str(&path.to_string_lossy());
            let url = NSURL::fileURLWithPath_isDirectory(&path, true);
            controller.noteNewRecentDocumentURL(&url);
        });
        if let Err(error) = result {
            eprintln!("Could not add recent Lumi IDE project: {error}");
        }
    }

    #[cfg(not(target_os = "macos"))]
    let _ = (app, path);
}
