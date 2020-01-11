use http::header::{HeaderName, HeaderValue};
use http::{StatusCode, Version};

// when in doubt - refer to https://www.w3.org/Protocols/HTTP/1.1/draft-ietf-http-v11-spec-01

pub fn parse_status_line(bytes: &[u8]) -> Option<(Version, StatusCode)> {
  // https://www.w3.org/Protocols/HTTP/1.1/draft-ietf-http-v11-spec-01#Status-Line
  let mut p = ParserHelper::new(bytes);
  p.take_seq(b"HTTP/1.")?;
  let version = match p.take_digit()? {
    0 => Version::HTTP_10,
    1 => Version::HTTP_11,
    _ => return None,
  };
  p.take_seq(b" ")?;
  let status_code = StatusCode::from_bytes(p.take(3)?).ok()?;
  p.take_seq(b" ")?;
  Some((version, status_code))
}

pub fn parse_header(bytes: &[u8]) -> Option<(HeaderName, HeaderValue)> {
  // https://www.w3.org/Protocols/HTTP/1.1/draft-ietf-http-v11-spec-01#Message-Headers
  let mut p = ParserHelper::new(bytes);
  let name = HeaderName::from_bytes(p.take_while(is_token_char)?).ok()?;
  p.take_seq(b":");
  p.take_optional_while(is_whitespace_char);
  let value =
    HeaderValue::from_bytes(p.take_while(|b| !is_linebreak_char(b))?).ok()?;
  Some((name, value))
}

fn is_control_char(b: u8) -> bool {
  b <= 31 || b == b'\x7f'
}

fn is_token_char(b: u8) -> bool {
  !is_control_char(b) && !is_special_char(b)
}

#[rustfmt::skip]
fn is_special_char(b: u8) -> bool {
  b == b'(' || b == b')' || b == b'<'  || b == b'>' || b == b'@' || b == b',' ||
  b == b';' || b == b':' || b == b'\\' || b == b'"' || b == b'/' || b == b'[' ||
  b == b']' || b == b'?' || b == b'='  || b == b'{' || b == b'}' || b == b' ' ||
  b == b'\t'
}

fn is_whitespace_char(b: u8) -> bool {
  b == b' ' || b == b'\t'
}

fn is_linebreak_char(b: u8) -> bool {
  b == b'\n' || b == b'\r'
}

#[derive(Debug)]
pub struct ParserHelper<'a> {
  bytes: &'a [u8],
  index: usize,
}

impl<'a> ParserHelper<'a> {
  pub fn new(bytes: &'a [u8]) -> Self {
    Self { bytes, index: 0 }
  }

  pub fn take_map<F, T>(&mut self, count: usize, f: F) -> Option<T>
  where
    F: FnOnce(&'a [u8]) -> Option<T>,
  {
    if self.index + count > self.bytes.len() {
      return None;
    }
    let bytes = &self.bytes[self.index..self.index + count];
    let result = f(bytes);
    if result.is_some() {
      self.index += count;
    }
    result
  }

  pub fn take_optional_while<F>(&mut self, mut predicate: F) -> &'a [u8]
  where
    F: FnMut(u8) -> bool,
  {
    let start_index = self.index;
    while self.index < self.bytes.len() && predicate(self.bytes[self.index]) {
      self.index += 1;
    }
    &self.bytes[start_index..self.index]
  }

  pub fn take_while<F>(&mut self, predicate: F) -> Option<&'a [u8]>
  where
    F: FnMut(u8) -> bool,
  {
    match self.take_optional_while(predicate) {
      [] => None,
      bytes => Some(bytes),
    }
  }

  pub fn take(&mut self, count: usize) -> Option<&'a [u8]> {
    self.take_map(count, Some)
  }

  pub fn take_seq(&mut self, expected: &[u8]) -> Option<&[u8]> {
    self.take_map(expected.len(), |bytes| {
      if bytes == expected {
        Some(bytes)
      } else {
        None
      }
    })
  }

  pub fn take_digit(&mut self) -> Option<u8> {
    self.take_map(1, |bytes| {
      let digit: u32 = (bytes[0] as char).to_digit(10)?;
      Some(digit as u8)
    })
  }

  pub fn take_usize(&mut self) -> Option<usize> {
    let mut result = self.take_digit()? as usize;
    while let Some(digit) = self.take_digit() {
      result = result.checked_mul(10)?.checked_add(digit as usize)?;
    }
    Some(result)
  }
}
