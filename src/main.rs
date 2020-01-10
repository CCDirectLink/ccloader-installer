use curl::easy::{Easy as Curl, List as CurlList};
use flate2::bufread::GzDecoder;
use serde_json::Value as JsonValue;
use std::path::{Path, PathBuf};
use std::str;
use tar::Archive;

mod native_ui;

const CCLOADER_GITHUB_API_RELEASE_URL: &str =
  // "https://api.github.com/repos/dmitmel/CCLoader/releases/latest";
  "http://localhost:8080/latest.json";

fn main() {
  curl::init();
  native_ui::init();

  let mut client = Curl::new();
  client.fail_on_error(true).unwrap();
  client.follow_location(true).unwrap();
  client
    .useragent(&format!(
      "{} v{} by dmitmel",
      env!("CARGO_PKG_NAME"),
      env!("CARGO_PKG_VERSION")
    ))
    .unwrap();

  client.url(CCLOADER_GITHUB_API_RELEASE_URL).unwrap();
  client
    .http_headers({
      let mut http_headers = CurlList::new();
      http_headers.append("Accept: application/vnd.github.v3+json").unwrap();
      http_headers
    })
    .unwrap();
  let release_json_bytes = {
    let mut body: Vec<u8> = Vec::new();
    let mut transfer = client.transfer();
    transfer
      .write_function(|chunk| {
        body.extend_from_slice(chunk);
        Ok(chunk.len())
      })
      .unwrap();
    transfer.perform().unwrap();
    drop(transfer);
    body
  };

  let release_data: JsonValue =
    serde_json::from_slice(&release_json_bytes).unwrap();
  let ccloader_download_url =
    get_download_url_from_release_data(&release_data).unwrap();
  println!("{}", ccloader_download_url);

  client.url(ccloader_download_url).unwrap();
  client
    .http_headers({
      let mut http_headers = CurlList::new();
      http_headers.append("Accept: */*").unwrap();
      http_headers
    })
    .unwrap();
  let compressed_archive_data = {
    let mut body: Vec<u8> = Vec::new();
    let mut transfer = client.transfer();
    transfer
      .write_function(|chunk| {
        body.extend_from_slice(chunk);
        Ok(chunk.len())
      })
      .unwrap();
    transfer.perform().unwrap();
    drop(transfer);
    body
  };

  let mut decoder = GzDecoder::new(&compressed_archive_data[..]);
  let mut archive = Archive::new(&mut decoder);
  archive.set_preserve_permissions(true);

  for entry in archive.entries().unwrap() {
    let entry = entry.unwrap();
    let header = entry.header();
    println!("{:?} {:?}", header.path().unwrap(), header.size().unwrap());
  }

  // archive.unpack("ccloader").unwrap();

  // let alert_response =
  //   native_ui::show_alert(native_ui::AlertConfig {
  //     style: native_ui::AlertStyle::Info,
  //     title: "Welcome to CC Mod Manager Launcher".to_owned(),
  //     description: Some("This program starts CC Mod Manager by reusing nw.js bundled with CrossCode. However, it first needs to locate your CrossCode installation.".to_owned()),
  //     primary_button_text: "Try to autodetect CC".to_owned(),
  //     secondary_button_text: Some("Specify path to CC manually".to_owned()),
  //   });
  // if alert_response == None {
  //   return;
  // };

  // let try_to_autodetect =
  //   alert_response == Some(native_ui::AlertResponse::PrimaryButtonPressed);

  // if try_to_autodetect {
  //   eprintln!("trying to autodetect CrossCode installation path");
  //   if let Some(game_path) = autodetect_game_location() {
  //     eprintln!("{:?}", game_path);
  //   }
  // }

  // while let Some(path) = native_ui::open_pick_folder_dialog() {
  //   if is_game_installed_in(&path) {
  //     println!("{}", path.display());
  //     break;
  //   } else {
  //     let alert_response = native_ui::show_alert(native_ui::AlertConfig {
  //       style: native_ui::AlertStyle::Problem,
  //       title:
  //         "Couldn't detect a CrossCode installation here. Please, try again."
  //           .to_owned(),
  //       description: None,
  //       primary_button_text: "Specify path to CC manually".to_owned(),
  //       secondary_button_text: Some("Exit".to_owned()),
  //     });
  //     if alert_response != Some(native_ui::AlertResponse::PrimaryButtonPressed)
  //     {
  //       break;
  //     }
  //   }
  // }
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

fn get_download_url_from_release_data(release: &JsonValue) -> Option<&str> {
  let assets: &Vec<JsonValue> = release["assets"].as_array()?;
  let ccloader_asset: &JsonValue = assets.iter().find(|asset| {
    asset["name"].as_str().map_or(false, |name| name.starts_with("ccloader_"))
  })?;
  let url: &str = ccloader_asset["browser_download_url"].as_str()?;
  Some(url)
}
