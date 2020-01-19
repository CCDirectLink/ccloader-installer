use ::curl::easy as curl;
use ::curl::Error as CurlError;
use log::info;

pub use http::header::{self, HeaderMap, HeaderName, HeaderValue};
pub use http::{Method, StatusCode, Uri, Version};

use crate::ascii_to_int::ascii_to_int;

mod parser;

pub type Body = Vec<u8>;
pub type Request<T = Body> = http::Request<T>;
pub type Response<T = Body> = http::Response<T>;

#[derive(Debug)]
pub struct HttpClient {
  curl: curl::Easy2<Handler>,
}

impl HttpClient {
  pub fn new() -> Self {
    HttpClient { curl: curl::Easy2::new(Handler::new()) }
  }

  pub fn send(&mut self, request: Request) -> Result<Response, CurlError> {
    info!(
      "request: {} {} {:?} {:?}",
      request.method(),
      request.uri(),
      request.version(),
      request.headers()
    );

    self.curl.reset();
    self.configure_curl_default()?;
    self.configure_curl_for_request(&request)?;
    self.curl.perform()?;

    let handler: &mut Handler = self.curl.get_mut();

    let mut response =
      Response::new(handler.response_body.take().unwrap_or_default());
    if let Some(version) = handler.response_version.take() {
      *response.version_mut() = version;
    }
    if let Some(headers) = handler.response_headers.take() {
      *response.headers_mut() = headers;
    }

    *response.status_mut() =
      StatusCode::from_u16(if request.method() == Method::CONNECT {
        self.curl.http_connectcode()?
      } else {
        self.curl.response_code()?
      } as u16)
      .unwrap();

    info!(
      "response: {:?} {} {:?}",
      response.version(),
      response.status(),
      response.headers()
    );

    Ok(response)
  }

  fn configure_curl_default(&mut self) -> Result<(), CurlError> {
    self.curl.follow_location(true)?;
    // self.curl.fail_on_error(true)?;
    self.curl.useragent(&format!(
      "{}/{} (by @dmitmel)",
      env!("CARGO_PKG_NAME"),
      env!("CARGO_PKG_VERSION")
    ))?;
    Ok(())
  }

  fn configure_curl_for_request(
    &mut self,
    request: &Request,
  ) -> Result<(), CurlError> {
    // taken from https://github.com/sagebind/isahc/blob/9909eda428bd87e8dbad7a0edba4b532b519c6a7/src/client.rs#L758-L890

    let body_length = request.body().len();
    let has_body = body_length > 0;

    match (request.method(), has_body) {
      (&Method::GET, false) => {
        // a normal GET request
        self.curl.get(true)?;
      }
      (&Method::HEAD, _) => {
        // HEAD requests do not wait for a response payload
        self.curl.upload(has_body)?;
        self.curl.nobody(true)?;
        self.curl.custom_request("HEAD")?;
      }
      (&Method::POST, _) => {
        // POST requests have special redirect behavior
        self.curl.post(true)?;
      }
      (&Method::PUT, _) => {
        // a normal PUT request
        self.curl.upload(true)?;
      }
      (method, _) => {
        // otherwise treat request like a GET or PUT
        self.curl.upload(has_body)?;
        self.curl.custom_request(method.as_str())?;
      }
    }

    self.curl.url(&request.uri().to_string())?;

    self.curl.http_version(match request.version() {
      http::Version::HTTP_10 => curl::HttpVersion::V10,
      http::Version::HTTP_11 => curl::HttpVersion::V11,
      _ => panic!("only HTTP/1.0 and HTTP/1.1 are supported"),
    })?;

    self.curl.accept_encoding(
      request
        .headers()
        .get(header::ACCEPT_ENCODING)
        .map(|value| value.to_str().unwrap())
        // empty string tells curl to fill in all supported encodings
        .unwrap_or(""),
    )?;

    let mut headers = curl::List::new();
    for (name, value) in request.headers().iter() {
      let header = format!("{}: {}", name.as_str(), value.to_str().unwrap());
      headers.append(&header)?;
    }
    self.curl.http_headers(headers)?;

    if has_body {
      let len = try_get_content_length_from_headers(request.headers())
        .unwrap_or(body_length);

      if request.method() == Method::POST {
        self.curl.post_field_size(len as u64)?;
      } else {
        self.curl.in_filesize(len as u64)?;
      }
    }

    Ok(())
  }
}

#[derive(Debug)]
struct Handler {
  response_version: Option<Version>,
  response_status_code: Option<StatusCode>,
  response_headers: Option<HeaderMap>,
  response_body: Option<Vec<u8>>,
}

impl Handler {
  fn new() -> Self {
    Self {
      response_version: None,
      response_status_code: None,
      response_headers: None,
      response_body: None,
    }
  }
}

impl curl::Handler for Handler {
  fn header(&mut self, data: &[u8]) -> bool {
    // this part was influenced by https://github.com/sagebind/isahc/blob/969b0800b5ab9119e2f72532a7522247bc639c2f/src/handler.rs

    if let Some((version, status_code)) = parser::parse_status_line(data) {
      self.response_version = Some(version);
      self.response_status_code = Some(status_code);
      if let Some(headers) = self.response_headers.as_mut() {
        headers.clear();
      }
    } else if let Some((name, value)) = parser::parse_header(data) {
      let headers = self.response_headers.get_or_insert_with(HeaderMap::new);
      headers.insert(name, value);
    } else if data != b"\r\n" && data != b"\n" && data != b"\r" {
      return false;
    }

    true
  }

  fn write(&mut self, chunk: &[u8]) -> Result<usize, curl::WriteError> {
    if self.response_body.is_none() {
      if let Some(headers) = self.response_headers.as_ref() {
        self.response_body = Some(Vec::with_capacity(
          try_get_content_length_from_headers(headers)
            .unwrap_or_else(|| chunk.len()),
        ));
      }
    }
    let response_body = self.response_body.as_mut().unwrap();
    response_body.extend_from_slice(chunk);
    Ok(chunk.len())
  }
}

fn try_get_content_length_from_headers(headers: &HeaderMap) -> Option<usize> {
  if let Some(header_value) = headers.get(header::TRANSFER_ENCODING) {
    if header_value.as_bytes().to_ascii_lowercase() != b"identity" {
      return None;
    }
  }

  let header_value = headers.get(header::CONTENT_LENGTH)?;
  ascii_to_int(header_value.as_bytes())
}
