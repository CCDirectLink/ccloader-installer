#![allow(non_upper_case_globals)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};

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
use wio::wide::{FromWide, ToWide};

use super::{AlertButtons, AlertConfig, AlertIcon, AlertResponse};

// taken from https://github.com/xi-editor/druid/blob/bafa1b9cb0fe156e9800a17f5d5e751be4ef6286/druid-shell/src/platform/windows/dialog.rs#L44-L46
// TODO: remove these when they get added to winapi
DEFINE_GUID! {CLSID_FileOpenDialog,
0xDC1C_5A9C, 0xE88A, 0x4DDE, 0xA5, 0xA1, 0x60, 0xF8, 0x2A, 0x20, 0xAE, 0xF7}

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

  let text: Vec<u16> = match (config.title, config.description) {
    (title, Some(description)) => format!("{}\n\n{}", title, description),
    (title, None) => title,
  }
  .to_wide_null();
  let title: Vec<u16> = crate::PKG_NAME.to_wide_null();

  let window_type: UINT = match config.icon {
    AlertIcon::Info => MB_ICONINFORMATION,
    AlertIcon::Warning => MB_ICONWARNING,
    AlertIcon::Error => MB_ICONERROR,
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
    let filename: OsString = OsString::from_wide_ptr_null(display_name);
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

  let path: Vec<u16> = path.to_wide_null();
  let operation: Vec<u16> = "explore".to_wide_null();

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
