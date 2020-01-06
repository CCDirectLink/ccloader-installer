#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
use macos::*;

fn main() {
  if let Some(path) = open_pick_folder_dialog() {
    println!("{}", path.display());
  }
}
