use std::convert::TryFrom;
use std::fs;
use std::path::{Path, PathBuf};
use std::str;

use flate2::bufread::GzDecoder;
use lazy_static::lazy_static;
use log::{error, info, warn, LevelFilter};
use log4rs::config::{Appender, Config, Root};
use serde_json::Value as JsonValue;
use tar::Archive;

#[macro_use]
mod error;
use error::err_msg;

mod ascii_to_int;
mod fancy_logger;
mod http_client;
mod native_ui;

use error::{Result as AppResult, ResultExt};
use http_client::{Body, HttpClient, Request as HttpRequest, Uri};

const CCMODDB_DATA_URL: &str =
  "https://github.com/CCDirectLink/CCModDB/raw/master/npDatabase.json";

const BUG_REPORT_TEXT: &str =
  "Please, contact @dmitmel on either GitHub, CrossCode official Discord server, or CCDirectLink Discord server. Bugs can be reported at https://github.com/dmitmel/ccloader-installer/issues";

lazy_static! {
  static ref CCLOADER_DIR_PATH: &'static Path = Path::new("ccloader");
  static ref MODS_DIR_PATH: &'static Path = Path::new("assets/mods");
}

fn main() {
  curl::init();
  native_ui::init();

  log4rs::init_config({
    let log_file_name = format!("{}.log", env!("CARGO_PKG_NAME"));
    let log_file_path: PathBuf = match fancy_logger::logs_directory() {
      Some(logs_dir) => logs_dir.join(log_file_name),
      None => {
        eprintln!(
          "logs directory not found, using the current working directory instead",
        );
        PathBuf::from(log_file_name)
      },
    };

    let mut b = Config::builder();
    let mut r = Root::builder();

    const CONSOLE_APPENDER_NAME: &str = "console";
    b = b.appender(Appender::builder().build(
      CONSOLE_APPENDER_NAME,
      Box::new(fancy_logger::ConsoleAppender::new(Box::new(
        fancy_logger::Encoder,
      ))),
    ));
    r = r.appender(CONSOLE_APPENDER_NAME);

    const FILE_APPENDER_NAME: &str = "file";
    match fancy_logger::FileAppender::new(
      &log_file_path,
      Box::new(fancy_logger::Encoder),
    ) {
      Ok(file_appender) => {
        b = b.appender(
          Appender::builder()
            .build(FILE_APPENDER_NAME, Box::new(file_appender)),
        );
        r = r.appender(FILE_APPENDER_NAME);
      }
      Err(e) => {
        eprintln!(
          "couldn't open log file '{}' (continuing anyway): {}",
          log_file_path.display(),
          e
        );
      }
    }

    b.build(r.build(LevelFilter::Info)).unwrap()
  })
  // logger initialization can't fail because the only error which can occur
  // happens if you try to set the logger twice
  .unwrap();

  fancy_logger::set_panic_hook();

  if let Err(error) = try_run() {
    error!("{}", error);
    native_ui::show_alert(native_ui::AlertConfig {
      style: native_ui::AlertStyle::Problem,
      title: error,
      description: Some(BUG_REPORT_TEXT.to_owned()),
      primary_button_text: "OK".to_owned(),
      secondary_button_text: None,
    });
  }

  native_ui::shutdown();
}

fn try_run() -> AppResult<()> {
  let mut client = HttpClient::new();

  let game_data_dir = match ask_for_game_data_dir() {
    Some(p) => p,
    None => return Ok(()),
  };
  info!("game data dir = {}", game_data_dir.display());

  let ccloader_dir = game_data_dir.join(&*CCLOADER_DIR_PATH);
  if ccloader_dir.is_dir() {
    bail!("The game data directory already contains a CCLoader installation (updating CCLoader isn't supported yet)")
  }

  let user_wants_to_continue =
    ask_for_installation_confirmation(&game_data_dir);
  if !user_wants_to_continue {
    return Ok(());
  }

  let release_info = fetch_latest_release_info(&mut client)
    .context("Couldn't fetch the latest release information")?;

  info!("release info = {:?}", release_info);

  let compressed_archive_data =
    download_release_archive(&mut client, release_info.download_url)
      .context("Couldn't donwload the latest CCLoader release")?;

  unpack_release_archive(
    compressed_archive_data,
    &release_info.root_dir_path,
    &game_data_dir,
  )
  .context("Couldn't unpack the CCLoader release archive")?;

  patch_crosscode_assets(&game_data_dir)
    .context("Couldn't patch CrossCode assets")?;

  info!("installation finished successfully");

  show_installation_success_alert(&game_data_dir);

  Ok(())
}

fn ask_for_game_data_dir() -> Option<PathBuf> {
  use native_ui::*;

  let try_to_autodetect = match show_alert(AlertConfig {
    style: AlertStyle::Info,
    title: "Welcome to CCLoader installer".to_owned(),
    description: Some(
      "This program installs the CCLoader mod loader for CrossCode. However, it first needs to locate your CrossCode game data directory."
        .to_owned(),
    ),
    primary_button_text: "Try to autodetect CC".to_owned(),
    secondary_button_text: Some("Specify the game data path manually".to_owned()),
  }) {
    Some(AlertResponse::PrimaryButtonPressed) => true,
    None => return None,
    _ => false,
  };

  if try_to_autodetect {
    info!("trying to autodetect the game data directory");
    if let Some(p) = autodetect_game_data_dir() {
      return Some(p);
    } else {
      info!("autodetection failed");
      match show_alert(AlertConfig {
        style: AlertStyle::Problem,
        title: "Couldn't autodetect your CrossCode game data directory"
          .to_owned(),
        description: None,
        primary_button_text: "Specify the game data path manually".to_owned(),
        secondary_button_text: Some("Exit".to_owned()),
      }) {
        Some(AlertResponse::PrimaryButtonPressed) => {}
        _ => return None,
      }
    }
  }

  while let Some(path) = {
    info!("specifying path to the game data directory manually");
    open_pick_folder_dialog()
  } {
    if is_game_data_dir(&path) {
      return Some(path);
    } else {
      match show_alert(AlertConfig {
        style: AlertStyle::Problem,
        title:
          "Couldn't detect a CrossCode game data directory here. Please, try again."
            .to_owned(),
        description: None,
        primary_button_text: "Specify path to CC manually".to_owned(),
        secondary_button_text: Some("Exit".to_owned()),
      }) {
        Some(AlertResponse::PrimaryButtonPressed) => {}
        _ => break,
      }
    }
  }

  None
}

fn autodetect_game_data_dir() -> Option<PathBuf> {
  possible_game_data_locations().into_iter().find(|path| is_game_data_dir(path))
}

fn is_game_data_dir(path: &Path) -> bool {
  info!("checking {}", path.display());
  path.is_dir()
    && path.join("package.json").is_file()
    && path.join("assets").is_dir()
    && path.join("assets/node-webkit.html").is_file()
}

#[cfg(target_os = "linux")]
fn get_possible_game_data_locations() -> Vec<PathBuf> {
  let mut result = Vec::with_capacity(1);
  if let Some(home) = dirs::home_dir() {
    result.push(home.join(".steam/steam/steamapps/common/CrossCode"));
  }
  result
}

#[cfg(target_os = "macos")]
fn possible_game_data_locations() -> Vec<PathBuf> {
  let mut result = Vec::with_capacity(1);
  if let Some(home) = dirs::home_dir() {
    result.push(home.join(
      "Library/Application Support/Steam/steamapps/common/CrossCode/CrossCode.app/Contents/Resources/app.nw",
    ));
  }
  result
}

#[cfg(target_os = "windows")]
fn get_possible_game_data_locations() -> Vec<PathBuf> {
  let mut result = vec![
    PathBuf::from("C:\\Program Files\\Steam\\steamapps\\common\\CrossCode"),
    PathBuf::from(
      "C:\\Program Files (x86)\\Steam\\steamapps\\common\\CrossCode",
    ),
  ];
  result
}

fn ask_for_installation_confirmation(game_data_dir: &Path) -> bool {
  use native_ui::*;

  show_alert(AlertConfig {
    style: AlertStyle::Info,
    title:
      "In order to install CCLoader, this installer has to modify CC asset files. Do you want to continue?"
        .to_owned(),
    description: Some(format!(
      "Path to the game data directory is {}",
      game_data_dir.display()
    )),
    primary_button_text: "Yes".to_owned(),
    secondary_button_text: Some("No, exit".to_owned()),
  }) == Some(AlertResponse::PrimaryButtonPressed)
}

#[derive(Debug)]
struct ReleaseInfo {
  download_url: Uri,
  root_dir_path: PathBuf,
}

fn fetch_latest_release_info(
  client: &mut HttpClient,
) -> AppResult<ReleaseInfo> {
  let response = client
    .send(HttpRequest::get(CCMODDB_DATA_URL).body(Vec::new()).unwrap())
    .context("network error")?;

  let status = response.status();
  if !status.is_success() {
    bail!("HTTP error: {}", status);
  }

  let release_data: JsonValue = serde_json::from_slice(&response.body())
    .context("invalid response received from CCModDB")?;

  try_ccmoddb_data_into_release_info(release_data)
    .ok_or_else(|| err_msg("invalid JSON data received from CCModDB"))
}

fn try_ccmoddb_data_into_release_info(
  mut data: JsonValue,
) -> Option<ReleaseInfo> {
  let package: &mut JsonValue = &mut data["ccloader"];
  let artifacts: &mut Vec<JsonValue> =
    package["installation"].as_array_mut()?;

  const ZIP_FILE_EXT: &str = ".zip";
  const TAR_GZ_FILE_EXT: &str = ".tar.gz";
  let main_artifact: &mut JsonValue =
    artifacts.iter_mut().find(|artifact| {
      artifact["type"].as_str() == Some("modZip")
        && artifact["source"]
          .as_str()
          .map_or(false, |s| s.starts_with("CCLoader-"))
        && artifact["url"].as_str().map_or(false, |s| s.ends_with(ZIP_FILE_EXT))
    })?;

  fn into_string(value: JsonValue) -> Option<String> {
    match value {
      JsonValue::String(s) => Some(s),
      _ => None,
    }
  }
  let mut download_url = into_string(main_artifact["url"].take())?;
  download_url.replace_range(
    download_url.len() - ZIP_FILE_EXT.len()..download_url.len(),
    TAR_GZ_FILE_EXT,
  );
  let root_dir_path = into_string(main_artifact["source"].take())?;

  Some(ReleaseInfo {
    download_url: Uri::try_from(download_url).ok()?,
    root_dir_path: PathBuf::from(root_dir_path),
  })
}

fn download_release_archive(
  client: &mut HttpClient,
  download_url: Uri,
) -> AppResult<Body> {
  let response = client
    .send(HttpRequest::get(download_url).body(Vec::new()).unwrap())
    .context("network error")?;

  let status = response.status();
  if !status.is_success() {
    bail!("HTTP error: {}", status);
  }

  Ok(response.into_body())
}

fn unpack_release_archive(
  compressed_archive_data: Vec<u8>,
  archive_root_dir_path: &Path,
  game_data_dir: &Path,
) -> AppResult<()> {
  let mut decoder = GzDecoder::new(&compressed_archive_data[..]);
  let mut archive = Archive::new(&mut decoder);
  archive.set_preserve_permissions(true);

  info!("unpacking the release archive to {}", game_data_dir.display());

  let unpacked_temporary_dir = game_data_dir.join(archive_root_dir_path);
  fs::create_dir_all(&unpacked_temporary_dir).with_context(|_| {
    format!("couldn't create directory '{}'", archive_root_dir_path.display())
  })?;

  for entry in archive.entries().context("archive error")? {
    let mut entry = entry.context("archive read I/O error")?;
    if let Ok(entry_path) = entry.path() {
      let entry_path: PathBuf = entry_path.into_owned();
      if let Ok(rel_path) = entry_path.strip_prefix(archive_root_dir_path) {
        if !(rel_path.starts_with(&*CCLOADER_DIR_PATH)
          || rel_path.starts_with(&*MODS_DIR_PATH))
        {
          continue;
        }

        info!("unpacking {}", rel_path.display());
        let was_unpacked: bool =
          entry.unpack_in(game_data_dir).context("archive unpack I/O error")?;
        if !was_unpacked {
          continue;
        }
      }
    }
  }

  let install = |rel_path: &Path| -> AppResult<()> {
    info!("installing {}", rel_path.display());
    fs::rename(
      unpacked_temporary_dir.join(rel_path),
      game_data_dir.join(rel_path),
    )
    .with_context(|_| format!("couldn't install '{}'", rel_path.display()))
  };

  install(&*CCLOADER_DIR_PATH)?;

  let mods_dir = game_data_dir.join(&*MODS_DIR_PATH);
  fs::create_dir_all(&mods_dir).with_context(|_| {
    format!("couldn't create directory '{}'", mods_dir.display())
  })?;
  for entry in fs::read_dir(unpacked_temporary_dir.join(&*MODS_DIR_PATH))
    .context("couldn't get the contents of the built-in mods directory")?
  {
    let entry = entry
      .context("couldn't get the contents of the built-in mods directory")?;
    if let Ok(file_type) = entry.file_type() {
      if file_type.is_dir() {
        let rel_path = MODS_DIR_PATH.join(entry.file_name());
        if !game_data_dir.join(&rel_path).is_dir() {
          install(&rel_path)?;
        } else {
          warn!("{} has already been installed, skipping", rel_path.display());
        }
      }
    }
  }

  fs::remove_dir_all(&unpacked_temporary_dir).with_context(|_| {
    format!("couldn't delete directory '{}'", archive_root_dir_path.display())
  })?;

  Ok(())
}

fn patch_crosscode_assets(game_data_dir: &Path) -> AppResult<()> {
  use std::fs::{File, OpenOptions};
  use std::io::{Seek, SeekFrom};

  let package_json_path = game_data_dir.join("package.json");
  info!("patching {}", package_json_path.display());

  let mut package_json_file: File = OpenOptions::new()
    .create(false)
    .read(true)
    .write(true)
    .open(package_json_path)
    .context("couldn't open package.json")?;

  let mut package_json_data: JsonValue =
    serde_json::from_reader(&mut package_json_file)
      .context("couldn't read package.json")?;
  if !package_json_data.is_object() {
    bail!("data in package.json is invalid");
  }

  package_json_data["main"] = JsonValue::String(format!(
    "{}/index.html",
    // unwrap is used because a) this is a compile time constant, so I can
    // guarantee that it's properly encoded because b) JSON strings can contain
    // only valid Unicode characters
    CCLOADER_DIR_PATH.to_str().unwrap()
  ));

  // truncate the file, then overwrite it with the patched data

  // set_len can return an error only if the file isn't opened for writing, or
  // the desired length would cause an integer overflow
  package_json_file.set_len(0).unwrap();
  // seek can fail only when called with a negative offset
  package_json_file.seek(SeekFrom::Start(0)).unwrap();

  serde_json::to_writer_pretty(&mut package_json_file, &package_json_data)
    .context("couldn't write patched package.json")?;

  Ok(())
}

fn show_installation_success_alert(game_data_dir: &Path) {
  use native_ui::*;
  if let Some(AlertResponse::PrimaryButtonPressed) = show_alert(AlertConfig {
    style: AlertStyle::Info,
    title: "CCLoader has been successfully installed!".to_owned(),
    description: None,
    primary_button_text: "Open the mods directory".to_owned(),
    secondary_button_text: Some("Exit".to_owned()),
  }) {
    open_path(&game_data_dir.join(&*MODS_DIR_PATH))
  }
}
