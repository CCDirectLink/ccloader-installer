#[cfg(target_os = "windows")]
fn main() {
  embed_resource::compile("ccloader-installer.rc");
}

#[cfg(not(target_os = "windows"))]
fn main() {}
