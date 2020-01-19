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

mod ascii_to_int;
mod error;
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
    std::process::exit(1);
  }
}

fn try_run() -> AppResult<()> {
  let mut client = HttpClient::new();

  let ccloader_download_url = fetch_latest_release_download_url(&mut client)
    .context("Couldn't fetch the latest release information")?;

  info!("release URL = {}", ccloader_download_url);

  let compressed_archive_data =
    download_release_archive(&mut client, ccloader_download_url)
      .context("Couldn't donwload the latest CCLoader release")?;

  unpack_release_archive(compressed_archive_data)
    .context("Couldn't unpack the CCLoader release archive")?;

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

  Ok(())
}

fn autodetect_game_location() -> Option<PathBuf> {
  get_possible_game_locations().into_iter().find(|path| {
    info!("checking {}", path.display());
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
      return Err(format!(
        "GitHub API ratelimit exceeded, please try again in {}",
        time,
      ));
    }
  }

  if !status.is_success() {
    return Err(format!("HTTP error: {}", status));
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
    return Err(format!("HTTP error: {}", status));
  }

  Ok(response.into_body())
}

fn unpack_release_archive(compressed_archive_data: Vec<u8>) -> AppResult<()> {
  let mut decoder = GzDecoder::new(&compressed_archive_data[..]);
  let mut archive = Archive::new(&mut decoder);
  archive.set_preserve_permissions(true);

  for entry in archive.entries().context("archive error")? {
    if let Err(err) = entry.as_ref() {
      error!("{:?}", err);
    }
    let entry = entry.context("I/O error")?;
    let header = entry.header();
    // info!("{:?} {:?}", header.path().unwrap(), header.size().unwrap());
  }

  // archive.unpack("ccloader").unwrap();

  Ok(())
}
