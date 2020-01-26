#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

#[derive(Debug, Clone)]
pub struct AlertConfig {
  pub style: AlertStyle,
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
pub enum AlertStyle {
  Info,
  Problem,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AlertResponse {
  Button1Pressed,
  Button2Pressed,
  Button3Pressed,
}
