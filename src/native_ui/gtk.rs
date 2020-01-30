use std::convert::TryInto;
use std::ffi::{CStr, CString, OsString};
use std::mem;
use std::os::raw::c_char;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};

use gio_sys::*;
use glib_sys::*;
use gobject_sys::*;
use gtk_sys::*;

use super::{AlertConfig, AlertIcon, AlertResponse};

pub fn init() {
  unsafe {
    gtk_init(null_mut(), null_mut());
  }
}

pub fn shutdown() {
  // unimplemented!()
}

pub fn show_alert(config: AlertConfig) -> Option<AlertResponse> {
  unsafe {
    let dialog: *mut GtkWidget = {
      let flags: GtkDialogFlags = 0;
      let type_: GtkMessageType = match config.icon {
        AlertIcon::Info => GTK_MESSAGE_INFO,
        AlertIcon::Warning => GTK_MESSAGE_WARNING,
        AlertIcon::Error => GTK_MESSAGE_ERROR,
      };
      let buttons: GtkButtonsType = GTK_BUTTONS_NONE;
      let message = CString::new(config.title).unwrap();
      gtk_message_dialog_new(
        null_mut::<GtkWindow>(),
        flags,
        type_,
        buttons,
        b"%s\0".as_ptr() as *const c_char,
        message.as_ptr(),
        null::<c_char>(),
      )
    };

    if let Some(description) = config.description {
      let description = CString::new(description).unwrap();
      let mut value: GValue = mem::zeroed();
      g_value_init(&mut value, G_TYPE_STRING);
      g_value_set_string(&mut value, description.as_ptr());
      g_object_set_property(
        dialog as *mut GObject,
        b"secondary-text\0".as_ptr() as *const c_char,
        &value,
      );
      g_value_unset(&mut value);
    }

    for (index, button_text) in
      config.buttons.to_strings().iter().enumerate().rev()
    {
      let button_text = CString::new(
        // use the first letter as accelerator
        format!("_{}", button_text),
      )
      .unwrap();
      // `try_into` is used here because from the standpoint of rustc a usize
      // index will not necessarily fit into a 32-bit variable (GtkResponseType
      // is an alias to c_int which is basically i32), but fortunatelly I don't
      // have 2^32 different AlertButtons variants, so an `unwrap` is fine here
      let response_id: GtkResponseType = index.try_into().unwrap();
      gtk_dialog_add_button(
        dialog as *mut GtkDialog,
        button_text.as_ptr(),
        response_id,
      );
    }
    gtk_dialog_set_default_response(dialog as *mut GtkDialog, 0);

    let response: GtkResponseType = gtk_dialog_run(dialog as *mut GtkDialog);

    gtk_widget_destroy(dialog);

    use AlertResponse::*;
    Some(match response {
      0 => Button1Pressed,
      1 => Button2Pressed,
      2 => Button3Pressed,
      _ => return None,
    })
  }
}

pub fn open_pick_folder_dialog() -> Option<PathBuf> {
  unsafe {
    let chooser: *mut GtkWidget = {
      gtk_file_chooser_dialog_new(
        null::<c_char>(),
        null_mut::<GtkWindow>(),
        GTK_FILE_CHOOSER_ACTION_SELECT_FOLDER,
        b"_Cancel\0".as_ptr() as *const c_char,
        GTK_RESPONSE_CANCEL,
        b"_Open\0".as_ptr() as *const c_char,
        GTK_RESPONSE_ACCEPT,
        null::<c_char>(),
      )
    };

    let mut result = None;

    let response: GtkResponseType = gtk_dialog_run(chooser as *mut GtkDialog);
    if response == GTK_RESPONSE_ACCEPT {
      let filename: *mut c_char =
        gtk_file_chooser_get_filename(chooser as *mut GtkFileChooser);
      if !filename.is_null() {
        let filename_bytes = CStr::from_ptr(filename).to_bytes().to_vec();
        g_free(filename as *mut _);
        result = Some(PathBuf::from(OsString::from_vec(filename_bytes)));
      }
    }

    gtk_widget_destroy(chooser);

    result
  }
}

pub fn open_path(path: &Path) {
  unsafe {
    let mut error: *mut GError = null_mut();

    let path = CString::new(path.as_os_str().as_bytes()).unwrap();
    let uri: *mut c_char = g_filename_to_uri(path.as_ptr(), null(), &mut error);
    if !error.is_null() {
      panic!("GTK error: {:?}", error)
    }

    g_app_info_launch_default_for_uri(
      uri,
      null_mut::<GAppLaunchContext>(),
      &mut error,
    );
    if !error.is_null() {
      panic!("GTK error: {:?}", error)
    }

    g_free(uri as *mut _);
  }
}
