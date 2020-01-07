use std::path::{Path, PathBuf};

mod native_ui;

fn main() {
  native_ui::init();

  eprintln!(
    "{:?}",
    native_ui::show_alert(native_ui::AlertConfig {
      style: native_ui::AlertStyle::Problem,
      title: "ACHTUNG".to_owned(),
      description: Some("PERFORMANCE WARNUNG".to_owned()),
      primary_button_text: "OK".to_owned(),
      secondary_button_text: Some("Cancel".to_owned()),
    })
  );

  eprintln!("trying to autodetect CrossCode installation path");
  if let Some(game_path) = get_possible_game_locations().iter().find(|path| {
    eprintln!("checking {}", path.display());
    is_game_installed_in(path)
  }) {
    println!("detected");
  }

  if let Some(path) = native_ui::open_pick_folder_dialog() {
    println!("{}", path.display());
  }
}

#[cfg(target_os = "linux")]
fn get_possible_game_locations() -> Vec<PathBuf> {
  let mut result = Vec::with_capacity(1);
  if let Some(home) = dirs::home_dir() {
    result.push(home.join(".steam/steam/steamapps/common/CrossCode"));
  }
  result
}

#[cfg(target_os = "macos")]
fn get_possible_game_locations() -> Vec<PathBuf> {
  let mut result = Vec::with_capacity(1);
  if let Some(home) = dirs::home_dir() {
    result.push(home.join(
      "Library/Application Support/Steam/steamapps/common/CrossCode/CrossCode.app/Contents/Resources/app.nw",
    ));
  }
  result
}

#[cfg(target_os = "windows")]
fn get_possible_game_locations() -> Vec<PathBuf> {
  let mut result = vec![
    PathBuf::from("C:\\Program Files/Steam/steamapps/common/CrossCode"),
    PathBuf::from("C:\\Program Files (x86)/Steam/steamapps/common/CrossCode"),
  ];
  result
}

fn is_game_installed_in(path: &Path) -> bool {
  path.is_dir()
    && path.join("package.json").is_file()
    && path.join("assets").is_dir()
}
