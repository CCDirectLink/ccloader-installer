use std::path::{Path, PathBuf};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos as sys;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use windows as sys;

#[derive(Debug, Clone)]
pub struct AlertConfig {
  pub icon: AlertIcon,
  pub title: String,
  pub description: Option<String>,
  pub buttons: AlertButtons,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AlertButtons {
  Ok,
  OkCancel,
  RetryCancel,
  YesNo,
  YesNoCancel,
}

impl AlertButtons {
  #[cfg_attr(target_os = "windows", allow(dead_code))]
  fn to_strings(self) -> &'static [&'static str] {
    use AlertButtons::*;
    match self {
      Ok => &["OK"],
      OkCancel => &["OK", "Cancel"],
      RetryCancel => &["Retry", "Cancel"],
      YesNo => &["Yes", "No"],
      YesNoCancel => &["Yes", "No", "Cancel"],
    }
  }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AlertIcon {
  Info,
  Warning,
  Error,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AlertResponse {
  Button1Pressed,
  Button2Pressed,
  Button3Pressed,
}

pub fn init() {
  sys::init()
}

pub fn shutdown() {
  sys::shutdown()
}

pub fn show_alert(config: AlertConfig) -> Option<AlertResponse> {
  sys::show_alert(config)
}

pub fn open_pick_folder_dialog() -> Option<PathBuf> {
  sys::open_pick_folder_dialog()
}

pub fn open_path(path: &Path) {
  sys::open_path(path)
}
