use std::error::Error;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufWriter, Stderr, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use encode::writer::console::ConsoleWriter;
use encode::writer::simple::SimpleWriter;
use log::{error, Level, LevelFilter, Record};
use log4rs::append::Append;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::{self, Color, Encode, Style};

#[cfg(target_os = "windows")]
const NEWLINE: &[u8] = b"\r\n";
#[cfg(not(target_os = "windows"))]
const NEWLINE: &[u8] = b"\n";

#[cfg(target_os = "macos")]
fn logs_directory() -> Option<PathBuf> {
  dirs::home_dir().map(|h| h.join("Library/Logs"))
}

#[cfg(not(target_os = "macos"))]
fn logs_directory() -> Option<PathBuf> {
  dirs::data_local_dir()
}

pub fn init() {
  log4rs::init_config({
    let log_file_name = format!("{}.log", crate::PKG_NAME);
    let log_file_path: PathBuf = match logs_directory() {
      Some(logs_dir) => logs_dir.join(log_file_name),
      None => {
        eprintln!(
          "logs directory not found, using the current working directory instead",
        );
        PathBuf::from(log_file_name)
      }
    };

    let mut b = Config::builder();
    let mut r = Root::builder();

    const CONSOLE_APPENDER_NAME: &str = "console";
    b = b.appender(Appender::builder().build(
      CONSOLE_APPENDER_NAME,
      Box::new(ConsoleAppender::new(Box::new(Encoder))),
    ));
    r = r.appender(CONSOLE_APPENDER_NAME);

    const FILE_APPENDER_NAME: &str = "file";
    match FileAppender::new(&log_file_path, Box::new(Encoder)) {
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
}

pub fn set_panic_hook() {
  std::panic::set_hook(Box::new(|info| {
    let backtrace = backtrace::Backtrace::new();

    let thread = std::thread::current();
    let thread_name = thread.name().unwrap_or("<unnamed>");

    let msg = match info.payload().downcast_ref::<&'static str>() {
      Some(s) => *s,
      None => match info.payload().downcast_ref::<String>() {
        Some(s) => &**s,
        None => "Box<Any>",
      },
    };

    match info.location() {
      Some(location) => {
        error!(
            target: "panic", "thread '{}' panicked at '{}', {}\n{:?}",
            thread_name,
            msg,
            location,
            backtrace
        );
      }
      None => error!(
          target: "panic",
          "thread '{}' panicked at '{}'\n{:?}",
          thread_name,
          msg,
          backtrace
      ),
    }
  }));
}

// ConsoleAppender has been partially taken from https://github.com/estk/log4rs/blob/c0a92f88eaf36e6bf59446fca1eaadeb6d2a578e/src/append/console.rs
// FileAppender has been partially taken from https://github.com/estk/log4rs/blob/c0a92f88eaf36e6bf59446fca1eaadeb6d2a578e/src/append/file.rs

pub struct ConsoleAppender {
  writer: ConsoleAppenderWriter,
  encoder: Box<dyn Encode>,
}

impl ConsoleAppender {
  pub fn new(encoder: Box<dyn Encode>) -> Self {
    Self {
      encoder,
      writer: match ConsoleWriter::stderr() {
        Some(writer) => ConsoleAppenderWriter::Tty(writer),
        None => ConsoleAppenderWriter::Raw(io::stderr()),
      },
    }
  }
}

enum ConsoleAppenderWriter {
  Tty(ConsoleWriter),
  Raw(Stderr),
}

impl fmt::Debug for ConsoleAppender {
  fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
    fmt.debug_struct("ConsoleAppender").field("encoder", &self.encoder).finish()
  }
}

impl Append for ConsoleAppender {
  fn append(
    &self,
    record: &Record,
  ) -> Result<(), Box<dyn Error + Sync + Send>> {
    match &self.writer {
      ConsoleAppenderWriter::Tty(w) => {
        let mut w = w.lock();
        self.encoder.encode(&mut w, record)?;
        w.flush()?;
      }
      ConsoleAppenderWriter::Raw(w) => {
        let mut w = SimpleWriter(w.lock());
        self.encoder.encode(&mut w, record)?;
        w.flush()?;
      }
    }
    Ok(())
  }

  fn flush(&self) {}
}

pub struct FileAppender {
  path: PathBuf,
  file: Mutex<SimpleWriter<BufWriter<File>>>,
  encoder: Box<dyn Encode>,
}

impl fmt::Debug for FileAppender {
  fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
    fmt
      .debug_struct("FileAppender")
      .field("file", &self.path)
      .field("encoder", &self.encoder)
      .finish()
  }
}

impl Append for FileAppender {
  fn append(
    &self,
    record: &Record,
  ) -> Result<(), Box<dyn Error + Sync + Send>> {
    let mut file = self.file.lock().unwrap_or_else(|e| e.into_inner());
    self.encoder.encode(&mut *file, record)?;
    file.flush()?;
    Ok(())
  }

  fn flush(&self) {}
}

impl FileAppender {
  pub fn new(path: &Path, encoder: Box<dyn Encode>) -> io::Result<Self> {
    if let Some(parent) = path.parent() {
      fs::create_dir_all(parent)?;
    }

    let file =
      OpenOptions::new().write(true).truncate(true).create(true).open(&path)?;

    Ok(Self {
      path: path.to_owned(),
      file: Mutex::new(SimpleWriter(BufWriter::with_capacity(1024, file))),
      encoder,
    })
  }
}

#[derive(Debug)]
pub struct Encoder;

impl Encode for Encoder {
  fn encode(
    &self,
    w: &mut dyn encode::Write,
    record: &Record,
  ) -> Result<(), Box<dyn Error + Sync + Send>> {
    w.write_all(b"[")?;
    write!(w, "{}", time::now_utc().rfc3339())?;
    let (level_color, level_str) = match record.level() {
      Level::Error => (Color::Red, "ERROR"),
      Level::Warn => (Color::Yellow, "WARN "),
      Level::Info => (Color::Green, "INFO "),
      Level::Debug => (Color::Magenta, "DEBUG "),
      Level::Trace => (Color::Blue, "TRACE"),
    };
    w.set_style(&Style::new().text(level_color))?;
    w.write_all(b" ")?;
    w.write_all(level_str.as_bytes())?;
    w.set_style(&Style::default())?;
    if let Some(module_path) = record.module_path() {
      w.write_all(b" ")?;
      w.write_all(module_path.as_bytes())?;
    }
    w.write_all(b"] ")?;
    w.write_fmt(*record.args())?;
    w.write_all(NEWLINE)?;
    Ok(())
  }
}
