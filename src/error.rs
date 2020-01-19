// inspired by the failure crate

use std::fmt::Display;
use std::result::Result as StdResult;

pub type Error = String;

pub type Result<T> = StdResult<T, Error>;

pub fn err_msg<D>(msg: D) -> Error
where
  D: Display,
{
  msg.to_string()
}

#[macro_export]
macro_rules! bail {
  ($e:expr) => {
    return Err($crate::error::err_msg($e));
  };
  ($fmt:expr, $($arg:tt)*) => {
    return Err($crate::error::err_msg(format!($fmt, $($arg)*)));
  };
}

pub trait ResultExt<T, E> {
  fn context<D>(self, context: D) -> Result<T>
  where
    D: Display;

  fn with_context<F, D>(self, f: F) -> Result<T>
  where
    F: FnOnce(&E) -> D,
    D: Display;
}

impl<T, E> ResultExt<T, E> for StdResult<T, E>
where
  E: Display,
{
  fn context<D>(self, context: D) -> Result<T>
  where
    D: Display,
  {
    self.with_context(|_| context)
  }

  fn with_context<F, D>(self, f: F) -> Result<T>
  where
    F: FnOnce(&E) -> D,
    D: Display,
  {
    self.map_err(|error| {
      let context = f(&error);
      format!("{}: {}", context, error)
    })
  }
}
