#![allow(clippy::let_unit_value)]
#![allow(clippy::enum_variant_names)]

use std::ffi::CStr;
use std::ffi::OsString;
use std::os::raw::c_char;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};

use cocoa::appkit::{
  NSApp, NSApplication, NSApplicationActivateIgnoringOtherApps,
  NSApplicationActivationPolicyRegular, NSRunningApplication,
};
use cocoa::base::{id, nil};
use cocoa::foundation::{NSAutoreleasePool, NSString, NSURL};
use objc::runtime::{NO, YES};
use objc::{class, msg_send, sel, sel_impl};

use super::{AlertConfig, AlertIcon, AlertResponse};

#[allow(dead_code)]
#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq)]
enum NSAlertStyle {
  NSWarningAlertStyle = 0,
  NSInformationalAlertStyle = 1,
  NSCriticalAlertStyle = 2,
}
use NSAlertStyle::*;

#[allow(dead_code)]
#[repr(isize)]
#[derive(Clone, Copy, Debug, PartialEq)]
enum NSModalResponse {
  NSModalResponseCancel = 0,
  NSModalResponseOK = 1,
  NSAlertFirstButtonReturn = 1000,
  NSAlertSecondButtonReturn = 1001,
  NSAlertThirdButtonReturn = 1002,
}
use NSModalResponse::*;

fn autorelease<T, F: FnOnce() -> T>(f: F) -> T {
  unsafe {
    let pool: id = NSAutoreleasePool::new(nil);
    let result = f();
    pool.drain();
    result
  }
}

pub fn init() {
  autorelease(|| unsafe {
    let app: id = NSApp();
    app.setActivationPolicy_(NSApplicationActivationPolicyRegular);
    app.finishLaunching();
  })
}

pub fn shutdown() {
  autorelease(|| unsafe {
    let app: id = NSApp();
    let _: () = msg_send![app, terminate: nil];
  })
}

fn request_focus() {
  autorelease(|| unsafe {
    let running_app: id = NSRunningApplication::currentApplication(nil);
    running_app.activateWithOptions_(NSApplicationActivateIgnoringOtherApps);
  })
}

pub fn show_alert(config: AlertConfig) -> Option<AlertResponse> {
  request_focus();

  autorelease(|| unsafe {
    let alert: id = msg_send![class!(NSAlert), alloc];
    let _: () = msg_send![alert, init];
    let _: () = msg_send![alert, autorelease];

    let mut ns_string: id;

    ns_string = NSString::alloc(nil).init_str(&config.title);
    let _: () = msg_send![alert, setMessageText: ns_string];
    if let Some(description) = config.description {
      ns_string = NSString::alloc(nil).init_str(&description);
      let _: () = msg_send![alert, setInformativeText: ns_string];
    }

    let ns_alert_style: NSAlertStyle = match config.icon {
      AlertIcon::Info => NSInformationalAlertStyle,
      AlertIcon::Warning | AlertIcon::Error => NSCriticalAlertStyle,
    };
    let _: () = msg_send![alert, setAlertStyle: ns_alert_style];

    for button_text in config.buttons.to_strings() {
      ns_string = NSString::alloc(nil).init_str(button_text);
      let _: () = msg_send![alert, addButtonWithTitle: ns_string];
    }

    let response: NSModalResponse = msg_send![alert, runModal];

    match response {
      NSAlertFirstButtonReturn => Some(AlertResponse::Button1Pressed),
      NSAlertSecondButtonReturn => Some(AlertResponse::Button2Pressed),
      NSAlertThirdButtonReturn => Some(AlertResponse::Button3Pressed),
      _ => None,
    }
  })
}

pub fn open_pick_folder_dialog() -> Option<PathBuf> {
  request_focus();

  autorelease(|| unsafe {
    let dialog: id = msg_send![class!(NSOpenPanel), openPanel];
    let _: () = msg_send![dialog, setAllowsMultipleSelection: NO];
    let _: () = msg_send![dialog, setCanChooseDirectories: YES];
    let _: () = msg_send![dialog, setCanCreateDirectories: YES];
    let _: () = msg_send![dialog, setCanChooseFiles: NO];
    let response: NSModalResponse = msg_send![dialog, runModal];

    if response == NSModalResponseOK {
      let url: id = msg_send![dialog, URL];
      let cstr: *const c_char = url.path().UTF8String();
      if !cstr.is_null() {
        let bytes: Vec<u8> = CStr::from_ptr(cstr).to_bytes().to_owned();
        return Some(PathBuf::from(OsString::from_vec(bytes)));
      }
    };

    None
  })
}

pub fn open_path(path: &Path) {
  autorelease(|| unsafe {
    let shared_workspace: id = msg_send![class!(NSWorkspace), sharedWorkspace];
    let ns_string: id = NSString::alloc(nil).init_str(
      // all paths on macOS must be valid Unicode (at least from my tests), so
      // I assume that the Path object here is UTF-8 encoded after converting
      // it from NSString to a Rust's UTF-8 string
      path.to_str().unwrap(),
    );
    let ns_url = NSURL::fileURLWithPath_(nil, ns_string);
    let _: () = msg_send![shared_workspace, openURL: ns_url];
  })
}
