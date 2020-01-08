use std::path::{Path, PathBuf};

mod native_ui;

fn main() {
  native_ui::init();

  let alert_response =
    native_ui::show_alert(native_ui::AlertConfig {
      style: native_ui::AlertStyle::Info,
      title: "Welcome to CC Mod Manager Launcher".to_owned(),
      description: Some("This program starts CC Mod Manager by reusing nw.js bundled with CrossCode. However, it first needs to locate your CrossCode installation.".to_owned()),
      primary_button_text: "Try to autodetect CC".to_owned(),
      secondary_button_text: Some("Specify path to CC manually".to_owned()),
    });
  if alert_response == None {
    return;
  };

  // let try_to_autodetect =
  //   alert_response == Some(native_ui::AlertResponse::PrimaryButtonPressed);

  // if try_to_autodetect {
  //   eprintln!("trying to autodetect CrossCode installation path");
  //   if let Some(game_path) = autodetect_game_location() {
  //     eprintln!("{:?}", game_path);
  //   }
  // }

  while let Some(path) = native_ui::open_pick_folder_dialog() {
    if is_game_installed_in(&path) {
      println!("{}", path.display());
      break;
    } else {
      let alert_response = native_ui::show_alert(native_ui::AlertConfig {
        style: native_ui::AlertStyle::Problem,
        title:
          "Couldn't detect a CrossCode installation here. Please, try again."
            .to_owned(),
        description: None,
        primary_button_text: "Specify path to CC manually".to_owned(),
        secondary_button_text: Some("Exit".to_owned()),
      });
      if alert_response != Some(native_ui::AlertResponse::PrimaryButtonPressed)
      {
        break;
      }
    }
  }
}

fn autodetect_game_location() -> Option<PathBuf> {
  get_possible_game_locations().into_iter().find(|path| {
    eprintln!("checking {}", path.display());
    is_game_installed_in(path)
  })
}

fn is_game_installed_in(path: &Path) -> bool {
  path.is_dir()
    && path.join("package.json").is_file()
    && path.join("assets").is_dir()
    && path.join("assets/node-webkit.html").is_file()
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
