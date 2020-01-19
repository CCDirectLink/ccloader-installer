use std::convert::TryFrom;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::str;
use std::time::{SystemTime, UNIX_EPOCH};

use log::{error, info, LevelFilter};
use log4rs::config::{Appender, Config, Root};

use flate2::bufread::GzDecoder;
use serde_json::Value as JsonValue;
use tar::Archive;

#[macro_use]
mod error;

mod ascii_to_int;
mod fancy_logger;
mod http_client;
mod native_ui;

use error::{Result as AppResult, ResultExt};
use http_client::{
  Body, HttpClient, Request as HttpRequest, Response as HttpResponse,
  StatusCode, Uri,
};

const CCLOADER_GITHUB_API_RELEASE_URL: &str =
  "https://api.github.com/repos/dmitmel/CCLoader/releases/latest"
  // "https://httpbin.org/redirect/1"
  // "http://localhost:8080/latest.json"
  ;

const BUG_REPORT_TEXT: &str =
  "Please, contact @dmitmel on either GitHub, CrossCode official Discord server, or CCDirectLink Discord server";

const CCLOADER_DIR_NAME_IN_ASSETS: &str = "ccloader";
const MODS_DIR_NAME_IN_ASSETS: &str = "assets/mods";

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
}

fn try_run() -> AppResult<()> {
  let mut client = HttpClient::new();

  let assets_dir = match ask_for_assets_dir() {
    Some(p) => p,
    None => return Ok(()),
  };
  info!("assets dir = {}", assets_dir.display());

  let ccloader_dir = assets_dir.join(CCLOADER_DIR_NAME_IN_ASSETS);
  if ccloader_dir.is_dir() {
    bail!("The assets directory already contains a CCLoader installation (updating CCLoader isn't supported yet)")
  }

  let user_wants_to_continue = ask_for_installation_confirmation(&assets_dir);
  if !user_wants_to_continue {
    return Ok(());
  }

  let ccloader_download_url = fetch_latest_release_download_url(&mut client)
    .context("Couldn't fetch the latest release information")?;

  info!("release URL = {}", ccloader_download_url);

  let compressed_archive_data =
    download_release_archive(&mut client, ccloader_download_url)
      .context("Couldn't donwload the latest CCLoader release")?;

  unpack_release_archive(compressed_archive_data, &ccloader_dir)
    .context("Couldn't unpack the CCLoader release archive")?;

  patch_crosscode_assets(&assets_dir)
    .context("Couldn't patch CrossCode assets")?;

  let mods_dir = assets_dir.join(MODS_DIR_NAME_IN_ASSETS);
  setup_mods_dir(&mods_dir, &ccloader_dir)
    .context("Couldn't setup the mods directory")?;

  info!("installation finished successfully");

  show_installation_success_alert(&mods_dir);

  Ok(())
}

fn ask_for_assets_dir() -> Option<PathBuf> {
  use native_ui::*;

  let try_to_autodetect = match show_alert(AlertConfig {
    style: AlertStyle::Info,
    title: "Welcome to CCLoader installer".to_owned(),
    description: Some(
      "This program installs the CCLoader mod loader for CrossCode. However, it first needs to locate your CrossCode assets directory."
        .to_owned(),
    ),
    primary_button_text: "Try to autodetect CC".to_owned(),
    secondary_button_text: Some("Specify the assets path manually".to_owned()),
  }) {
    Some(AlertResponse::PrimaryButtonPressed) => true,
    None => return None,
    _ => false,
  };

  if try_to_autodetect {
    info!("trying to autodetect the assets directory");
    if let Some(p) = autodetect_assets_dir() {
      return Some(p);
    } else {
      info!("autodetection failed");
      match show_alert(AlertConfig {
        style: AlertStyle::Problem,
        title: "Couldn't autodetect your CrossCode assets directory".to_owned(),
        description: None,
        primary_button_text: "Specify the assets path manually".to_owned(),
        secondary_button_text: Some("Exit".to_owned()),
      }) {
        Some(AlertResponse::PrimaryButtonPressed) => {}
        _ => return None,
      }
    }
  }

  while let Some(path) = {
    info!("specifying path to the assets directory manually");
    open_pick_folder_dialog()
  } {
    if is_assets_dir(&path) {
      return Some(path);
    } else {
      match show_alert(AlertConfig {
        style: AlertStyle::Problem,
        title:
          "Couldn't detect a CrossCode assets directory here. Please, try again."
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

fn autodetect_assets_dir() -> Option<PathBuf> {
  possible_assets_locations().into_iter().find(|path| is_assets_dir(path))
}

fn is_assets_dir(path: &Path) -> bool {
  info!("checking {}", path.display());
  path.is_dir()
    && path.join("package.json").is_file()
    && path.join("assets").is_dir()
    && path.join("assets/node-webkit.html").is_file()
}

#[cfg(target_os = "linux")]
fn get_possible_assets_locations() -> Vec<PathBuf> {
  let mut result = Vec::with_capacity(1);
  if let Some(home) = dirs::home_dir() {
    result.push(home.join(".steam/steam/steamapps/common/CrossCode"));
  }
  result
}

#[cfg(target_os = "macos")]
fn possible_assets_locations() -> Vec<PathBuf> {
  let mut result = Vec::with_capacity(1);
  if let Some(home) = dirs::home_dir() {
    result.push(home.join(
      "Library/Application Support/Steam/steamapps/common/CrossCode/CrossCode.app/Contents/Resources/app.nw",
    ));
  }
  result
}

#[cfg(target_os = "windows")]
fn get_possible_assets_locations() -> Vec<PathBuf> {
  let mut result = vec![
    PathBuf::from("C:\\Program Files/Steam/steamapps/common/CrossCode"),
    PathBuf::from("C:\\Program Files (x86)/Steam/steamapps/common/CrossCode"),
  ];
  result
}

fn ask_for_installation_confirmation(assets_dir: &Path) -> bool {
  use native_ui::*;

  show_alert(AlertConfig {
    style: AlertStyle::Info,
    title:
      "In order to install CCLoader, this installer has to modify CC asset files. Do you want to continue?"
        .to_owned(),
    description: Some(format!(
      "Path to the assets directory is {}",
      assets_dir.display()
    )),
    primary_button_text: "Yes".to_owned(),
    secondary_button_text: Some("No, exit".to_owned()),
  }) == Some(AlertResponse::PrimaryButtonPressed)
}

fn fetch_latest_release_download_url(
  client: &mut HttpClient,
) -> AppResult<Uri> {
  let response = client
    .send(
      HttpRequest::get(CCLOADER_GITHUB_API_RELEASE_URL)
        .header("Accept", "application/vnd.github.v3+json")
        .body(Vec::new())
        .unwrap(),
    )
    .context("network error")?;

  let status = response.status();

  info!("{}", String::from_utf8_lossy(&response.body()));

  if status == StatusCode::FORBIDDEN {
    // try to provide a more useful error message in case of ratelimits
    if let Some(time) = try_get_github_api_ratelimit_reset_human(&response) {
      bail!("GitHub API ratelimit exceeded, please try again in {}", time);
    }
  }

  if !status.is_success() {
    bail!("HTTP error: {}", status);
  }

  let release_data: JsonValue = serde_json::from_slice(&response.body())
    .context("invalid response received from GitHub API")?;
  let url_str: &str = get_download_url_from_release_data(&release_data)
    .ok_or("invalid JSON data received from GitHub API")?;
  let url: Uri = Uri::try_from(url_str)
    .context("invalid donwload URL received from GitHub API")?;

  Ok(url)
}

fn try_get_github_api_ratelimit_reset_human(
  response: &HttpResponse,
) -> Option<String> {
  let headers = response.headers();

  if headers.get("x-ratelimit-remaining")? != "0" {
    return None;
  }
  let ratelimit_reset: u64 =
    ascii_to_int::ascii_to_int(headers.get("x-ratelimit-reset")?.as_bytes())?;

  let duration_since_unix_epoch =
    SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
  let timestamp = duration_since_unix_epoch.as_secs();

  let remaining_time = ratelimit_reset.checked_sub(timestamp)?;
  let minutes = remaining_time / 60;
  let seconds = remaining_time % 60;

  let mut human_remaining_time = String::new();
  if minutes > 0 {
    write!(human_remaining_time, "{} minutes ", minutes).unwrap();
  }
  write!(human_remaining_time, "{} seconds", seconds).unwrap();

  Some(human_remaining_time)
}

fn get_download_url_from_release_data(release: &JsonValue) -> Option<&str> {
  let assets: &Vec<JsonValue> = release["assets"].as_array()?;
  let ccloader_asset: &JsonValue = assets.iter().find(|asset| {
    asset["name"].as_str().map_or(false, |name| name.starts_with("ccloader_"))
  })?;
  let url: &str = ccloader_asset["browser_download_url"].as_str()?;
  Some(url)
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
  ccloader_dir: &Path,
) -> AppResult<()> {
  let mut decoder = GzDecoder::new(&compressed_archive_data[..]);
  let mut archive = Archive::new(&mut decoder);
  archive.set_preserve_permissions(true);

  info!("unpacking the release archive to {}", ccloader_dir.display());

  for entry in archive.entries().context("archive error")? {
    let mut entry = entry.context("archive read I/O error")?;
    let header = entry.header();
    info!("unpacking {}", String::from_utf8_lossy(&header.path_bytes()));

    entry.unpack_in(ccloader_dir).context("archive unpack I/O error")?;
  }

  Ok(())
}

fn patch_crosscode_assets(assets_dir: &Path) -> AppResult<()> {
  use std::fs::{File, OpenOptions};
  use std::io::{Seek, SeekFrom};

  let package_json_path = assets_dir.join("package.json");
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

  package_json_data["main"] =
    JsonValue::String(format!("{}/index.html", CCLOADER_DIR_NAME_IN_ASSETS));

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

fn setup_mods_dir(mods_dir: &Path, ccloader_dir: &Path) -> AppResult<()> {
  use std::fs;
  use std::io;

  fs::create_dir_all(mods_dir).with_context(|_| {
    format!("couldn't create directory '{}'", mods_dir.display())
  })?;

  for entry in fs::read_dir(ccloader_dir.join("builtin-mods"))
    .context("couldn't get the contents of the built-in mods directory")?
  {
    let entry = entry
      .context("couldn't get the contents of the built-in mods directory")?;
    if let Ok(file_type) = entry.file_type() {
      if file_type.is_dir() {
        let name: std::ffi::OsString = entry.file_name();

        #[cfg(unix)]
        pub fn symlink_dir<P: AsRef<Path>, Q: AsRef<Path>>(
          src: P,
          dst: Q,
        ) -> io::Result<()> {
          std::os::unix::fs::symlink(src.as_ref(), dst.as_ref())
        }

        #[cfg(windows)]
        use std::os::windows::symlink_dir;

        if let Err(error) = symlink_dir(entry.path(), mods_dir.join(&name)) {
          if error.kind() != io::ErrorKind::AlreadyExists {
            return Err(error).context(format!(
              "couldn't create a link for built-in mod '{}'",
              name.to_string_lossy(),
            ));
          }
        }
      }
    }
  }

  Ok(())
}

fn show_installation_success_alert(mods_dir: &Path) {
  use native_ui::*;
  if let Some(AlertResponse::PrimaryButtonPressed) = show_alert(AlertConfig {
    style: AlertStyle::Info,
    title: "CCLoader has been successfully installed!".to_owned(),
    description: None,
    primary_button_text: "Open the mods directory".to_owned(),
    secondary_button_text: Some("Exit".to_owned()),
  }) {
    open_path(&mods_dir)
  }
}
