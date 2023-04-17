use std::path::PathBuf;

use appkit_nsworkspace_bindings::{
    INSRunningApplication, INSWorkspace, NSRunningApplication,
    NSWorkspace, NSWorkspace_NSWorkspaceRunningApplications, INSURL,
};
use cacao::{foundation::{NSArray, NSString}, appkit::Alert};
use crate::os::shared::ActiveWindowInfo;

pub fn output_panic_text(text: String) {
    println!("{}", &text);

    Alert::new("Panic", &text).show();
}

pub fn get_active_window_info() -> Option<ActiveWindowInfo> {
    unsafe {
        let workspace = NSWorkspace::sharedWorkspace();
        let running_apps = workspace.runningApplications();
        let ra = NSArray::from_retained(running_apps.0);
        let active_app: Option<ActiveWindowInfo> = ra
            .map(|item| -> NSRunningApplication { NSRunningApplication(item) })
            .iter()
            .fold(None, |prev, item| {
                if item.isActive() {
                    let exe = NSString::from_retained(item.executableURL().absoluteString().0)
                        .to_string();
                    let name = NSString::from_retained(item.localizedName().0).to_string();

                    Some(ActiveWindowInfo {
                        exe_path: Some(PathBuf::from(exe)),
                        window_name: Some(name),
                    })
                } else {
                    prev
                }
            });

        return active_app;
    }
}
