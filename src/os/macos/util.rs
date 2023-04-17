use appkit_nsworkspace_bindings::{INSWorkspace, NSWorkspace, NSWorkspace_NSWorkspaceRunningApplications, NSRunningApplication};

use crate::os::shared::ActiveWindowInfo;

pub fn output_panic_text(text: String) {
  println!("{}", &text);
}

pub fn get_active_window_info() -> ActiveWindowInfo {
  unsafe {
    let workspace = NSWorkspace::sharedWorkspace();
    let running_apps = workspace.runningApplications();
    let ra = cacao::foundation::NSArray::from_retained(running_apps.0);
    let vec: Vec<NSRunningApplication> = ra.map(|item| {
      item.into()
    });
    
    
  }
  

  todo!();
}