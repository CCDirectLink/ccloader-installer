# Development notes

(I hope I didn't forget anything important.)

## Building distributables

### Linux

Always link dynamically. libcurl can be assumed to be installed on virtually all distros (or at least Linux Mint), same with Zlib (which is a dependency of libcurl), and lastly GTK will be installed on any system with GUI (why would you use CCLoader installer on a headless system?). Plus GTK is a copyleft-library, so I wouldn't risk statically linking it into a non-GPL program.

Some graphical file managers incorrectly identify PIEs (position-independent executables), or at least used to (because as of writing this document this works correctly on my machine), as shared objects (if I recall correctly), as such to allow users to simply double-click the executable to start it, compilation of a non-PIE was needed. Here's one way to do this:

```sh
RUSTFLAGS='-C relocation-model=dynamic-no-pic' cargo build --release
```

Beware, however, that this used to break linking if the executable depended on procedural macros. For some reason this works now, though unfortunately I wasn't able to find much in my browser or shell history. Neither the solution I tried using before, nor the workaround I found, nor where I found it. Here's a (probably) relevant ticket though: rust-lang/cargo#5115.

### macOS

Make sure to dynamically link with system libcurl and Zlib, not their Homebrew counterparts. Don't forget to create an `.app` with [`cargo bundle`](https://github.com/burtonageo/cargo-bundle#cargo-bundle).

### Windows

Always link statically with both libcurl and Zlib as they are obviously not present in the standard Windows distribution. Assembly manifest injection is handled automatically, though I haven't figured out how to add an icon to the executable yet.

### Compressing archives

Always create tars with

```sh
tar --create --autho-compress --verbose --owner=0 --group=0 --file ccloader-installer_v1.2.3_linux.tar.gz ccloader-installer
```

or similar. The `--owner=0` and `--group=0` options remove UIDs and GIDs from the archived files.

To compress zips I personally use the good old `zip` command:

```sh
zip -r ccloader-installer_v1.2.3_windows.zip ccloader-installer.exe
```

Additionally, when compressing the archive for macOS don't forget to remove `.DS_Store` and `__MACOSX` meta-files. This can be done by either setting the `COPYFILE_DISABLE` environment variable, or passing the `--disable-copyfile` flag to the `tar` command. See <https://superuser.com/q/61185/1272235>.
