use std::ffi::CStr;
use std::ffi::OsString;
use std::os::raw::c_char;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;

use cocoa::appkit::{
  NSApplication, NSApplicationActivateIgnoringOtherApps,
  NSApplicationActivationPolicyRegular, NSRunningApplication,
};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSInteger, NSString, NSURL};
use objc::runtime::{NO, YES};
use objc::{class, msg_send, sel, sel_impl};

#[allow(clippy::let_unit_value)]
pub fn open_pick_folder_dialog() -> Option<PathBuf> {
  unsafe {
    let pool = NSAutoreleasePool::new(nil);

    let app = NSApplication::sharedApplication(nil);
    app.setActivationPolicy_(NSApplicationActivationPolicyRegular);
    app.finishLaunching();

    let running_app = NSRunningApplication::currentApplication(nil);
    running_app.activateWithOptions_(NSApplicationActivateIgnoringOtherApps);

    // let alert: id = msg_send![class!(NSAlert), alloc];
    // let _: () = msg_send![alert, init];
    // let _: () = msg_send![alert, autorelease];
    // let _: () =
    //   msg_send![alert, addButtonWithTitle: NSString::alloc(nil).init_str("OK")];
    // let _: () =
    //   msg_send![alert, setMessageText: NSString::alloc(nil).init_str("Hello")];
    // let _: () = msg_send![alert, setInformativeText: NSString::alloc(nil).init_str("World")];
    // let _: () = msg_send![alert, setAlertStyle: 1];
    // let result: NSInteger = msg_send![alert, runModal];

    let dialog: id = msg_send![class!(NSOpenPanel), openPanel];
    let _: () = msg_send![dialog, setAllowsMultipleSelection: NO];
    let _: () = msg_send![dialog, setCanChooseDirectories: YES];
    let _: () = msg_send![dialog, setCanCreateDirectories: YES];
    let _: () = msg_send![dialog, setCanChooseFiles: NO];
    let modal_response: NSInteger = msg_send![dialog, runModal];

    let mut result = None;

    if modal_response == /* NSModalResponseOK */ 1 {
      let url: id = msg_send![dialog, URL];
      let cstr: *const c_char = url.path().UTF8String();
      if !cstr.is_null() {
        let bytes: Vec<u8> = CStr::from_ptr(cstr).to_bytes().to_owned();
        result = Some(PathBuf::from(OsString::from_vec(
          CStr::from_ptr(cstr).to_owned().into_bytes(),
        )))
      }
    };

    pool.drain();

    result
  }
}
