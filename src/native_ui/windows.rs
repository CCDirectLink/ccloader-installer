#![allow(non_upper_case_globals)]

use std::ffi::{OsStr, OsString};
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};
use std::slice;

use winapi::shared::minwindef::{HINSTANCE, INT, LPVOID, UINT};
use winapi::shared::ntdef::{HRESULT, LPWSTR};
use winapi::shared::winerror::{
  ERROR_CANCELLED, HRESULT_FROM_WIN32, SUCCEEDED, S_FALSE, S_OK,
};
use winapi::shared::wtypesbase::CLSCTX_INPROC_SERVER;
use winapi::um::combaseapi::*;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::objbase::{COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE};
use winapi::um::shellapi::ShellExecuteW;
use winapi::um::shobjidl::*;
use winapi::um::shobjidl_core::*;
use winapi::um::winuser::*;
use winapi::{Interface, DEFINE_GUID};
use wio::com::ComPtr;

use super::{AlertButtons, AlertConfig, AlertResponse, AlertStyle};

// taken from https://github.com/xi-editor/druid/blob/bafa1b9cb0fe156e9800a17f5d5e751be4ef6286/druid-shell/src/platform/windows/dialog.rs#L44-L46
// TODO: remove these when they get added to winapi
DEFINE_GUID! {CLSID_FileOpenDialog,
0xDC1C_5A9C, 0xE88A, 0x4DDE, 0xA5, 0xA1, 0x60, 0xF8, 0x2A, 0x20, 0xAE, 0xF7}

fn to_wide_null(s: &OsStr) -> Vec<u16> {
  s.encode_wide()
    .inspect(|b| {
      if *b == 0 {
        panic!("nul byte found in data");
      }
    })
    .chain(std::iter::once(0))
    .collect()
}

pub fn init() {
  unsafe {
    // taken from https://github.com/microsoft/com-rs/blob/3e3abd1de1312b6ee535f9de75a8aeb12b42520a/src/runtime.rs#L26-L45
    match CoInitializeEx(
      null_mut(),
      COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE,
    ) {
      // S_OK indicates the runtime was initialized, S_FALSE means it was
      // initialized previously
      S_OK | S_FALSE => {}
      // any other result is considered an error here
      hr => check_hresult(hr),
    }
  }
}

pub fn shutdown() {
  unsafe {
    // taken from https://github.com/microsoft/com-rs/blob/3e3abd1de1312b6ee535f9de75a8aeb12b42520a/src/runtime.rs#L107-L111
    CoUninitialize()
  }
}

pub fn show_alert(config: AlertConfig) -> Option<AlertResponse> {
  // initially adapted from https://github.com/bekker/msgbox-rs/blob/520dd1b21f5d776ec1ce1396a79abd536cf49c67/src/windows.rs

  let text: Vec<u16> = to_wide_null(
    match (config.title, config.description) {
      (title, Some(description)) => format!("{}\n\n{}", title, description),
      (title, None) => title,
    }
    .as_ref(),
  );
  let title: Vec<u16> = to_wide_null(env!("CARGO_PKG_NAME").as_ref());

  let window_type: UINT = match config.style {
    AlertStyle::Problem => MB_ICONERROR,
    AlertStyle::Info => MB_ICONINFORMATION,
  } | match config.buttons {
    AlertButtons::Ok => MB_OK,
    AlertButtons::OkCancel => MB_OKCANCEL,
    AlertButtons::RetryCancel => MB_RETRYCANCEL,
    AlertButtons::YesNo => MB_YESNO,
    AlertButtons::YesNoCancel => MB_YESNOCANCEL,
  } | MB_SYSTEMMODAL;

  let response: INT = unsafe {
    MessageBoxW(null_mut(), text.as_ptr(), title.as_ptr(), window_type)
  };

  use AlertResponse::*;
  Some(match (config.buttons, response) {
    (_, 0) => {
      panic!("MessageBoxW error (DWORD): {:?}", unsafe { GetLastError() });
    }
    (AlertButtons::Ok, IDOK) => Button1Pressed,
    (AlertButtons::OkCancel, IDOK) => Button1Pressed,
    (AlertButtons::OkCancel, IDCANCEL) => Button2Pressed,
    (AlertButtons::RetryCancel, IDRETRY) => Button1Pressed,
    (AlertButtons::RetryCancel, IDCANCEL) => Button2Pressed,
    (AlertButtons::YesNo, IDYES) => Button1Pressed,
    (AlertButtons::YesNo, IDNO) => Button2Pressed,
    (AlertButtons::YesNoCancel, IDYES) => Button1Pressed,
    (AlertButtons::YesNoCancel, IDNO) => Button2Pressed,
    (AlertButtons::YesNoCancel, IDCANCEL) => Button3Pressed,
    _ => return None,
  })
}

pub fn open_pick_folder_dialog() -> Option<PathBuf> {
  unsafe {
    // taken from https://github.com/xi-editor/druid/blob/bafa1b9cb0fe156e9800a17f5d5e751be4ef6286/druid-shell/src/platform/windows/dialog.rs#L74-L155

    let mut file_dialog: *mut IFileDialog = null_mut();
    check_hresult(CoCreateInstance(
      &CLSID_FileOpenDialog,
      null_mut(),
      CLSCTX_INPROC_SERVER,
      &IFileOpenDialog::uuidof(),
      &mut file_dialog as *mut *mut IFileDialog as *mut LPVOID,
    ));
    let file_dialog = ComPtr::from_raw(file_dialog);

    check_hresult(file_dialog.SetOptions(FOS_PICKFOLDERS));

    let hr = file_dialog.Show(null_mut());
    if hr == HRESULT_FROM_WIN32(ERROR_CANCELLED) {
      return None;
    }
    check_hresult(hr);

    let mut result_ptr: *mut IShellItem = null_mut();
    check_hresult(file_dialog.GetResult(&mut result_ptr));
    let shell_item = ComPtr::from_raw(result_ptr);

    let mut display_name: LPWSTR = null_mut();
    check_hresult(
      shell_item.GetDisplayName(SIGDN_FILESYSPATH, &mut display_name),
    );
    let filename: OsString = OsStringExt::from_wide({
      let mut len = 0;
      while *display_name.offset(len) != 0 {
        len += 1;
      }
      slice::from_raw_parts(display_name, len as usize)
    });
    CoTaskMemFree(display_name as LPVOID);

    Some(PathBuf::from(filename))
  }
}

pub fn check_hresult(hr: HRESULT) {
  if !SUCCEEDED(hr) {
    panic!("WinAPI error (HRESULT): {:?}", hr);
  }
}

pub fn open_path(path: &Path) {
  // based on https://github.com/Byron/open-rs/blob/9d9e40cc9b68266652a5ac21915b558b812ee444/src/lib.rs#L72-L99

  let path: Vec<u16> = to_wide_null(path.as_os_str());
  let operation: Vec<u16> = to_wide_null("explore".as_ref());

  let result: HINSTANCE = unsafe {
    ShellExecuteW(
      null_mut(),
      operation.as_ptr(),
      path.as_ptr(),
      null(),
      null(),
      SW_SHOW,
    )
  };

  if result as INT <= 32 {
    panic!("ShellExecuteW error (HINSTANCE): {:?}", result);
  }
}
