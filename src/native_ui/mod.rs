#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::*;

#[derive(Debug, Clone)]
pub struct AlertConfig {
  pub style: AlertStyle,
  pub title: String,
  pub description: Option<String>,
  pub primary_button_text: String,
  pub secondary_button_text: Option<String>,
}

#[derive(Debug, Copy, Clone)]
pub enum AlertStyle {
  Info,
  Problem,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum AlertResponse {
  PrimaryButtonPressed,
  SecondaryButtonPressed,
}
