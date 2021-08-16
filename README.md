# ccloader-installer

The official automatic installer for [CCLoader](https://github.com/CCDirectLink/CCLoader).

## Usage instructions

**NOTE:** Do not use the _"Download ZIP"_ button in the _"Clone or download"_ menu! _It downloads an archive of the source code, not the executable files!_

**NOTE 2:** I'll be using the word "directory" a lot. It has the same meaning as "folder", I use it just because that's my personal preference. Also, I prefer forward slashes (`/`) to backward ones (`\`) in file paths.

1. Go to the [latest release](https://github.com/CCDirectLink/ccloader-installer/releases/latest) page.
2. Download an archive for your OS:
   - **MS Windows (64-bit, x86_64):** `ccloader-installer_vX.Y.Z_windows.zip` - contains `ccloader-installer.exe`
   - **macOS:** `ccloader-installer_vX.Y.Z_macos.tar.gz` - contains `ccloader-installer.app` (don't worry, macOS's standard Archive Utility application can handle this archive; [why tar.gz?](#why-targz-and-not-zip))
   - **Linux (64-bit, x86_64):** `ccloader-installer_vX.Y.Z_linux.tar.gz` - contains `ccloader-installer`
3. Unpack the archive, run the `ccloader-installer` executable file.
4. You will be presented with the choice to either _autodetect your CrossCode game data directory_ or _specify the path to it manually_ ([what is the "CrossCode game data directory"?](#what-is-the-crosscode-game-data-directory)). In most cases the autodetection algorithm should work perfectly fine. Please note that _you can't choose a game data directory which already contains a CCLoader installation_ - updating CCLoader isn't supported yet.
5. [The rest of the process is automatic](#what-does-the-installer-exactly-do) - the installer will make necessary changes to the game files.
6. After the installation is complete, you'll be presented with the option to open [the mods directory](#where-do-i-put-mods). It is recommended to remember the path to it or note it down somewhere.

## Manual CCLoader installation guide

**NOTE:** about the _"Download ZIP"_ button: for now, you can indeed use it to download and install CCLoader. However, this is not recommended: first of all, this might change in the future. Second, you'll be downloading the latest bleeding-edge development version of CCLoader - you shouldn't do this without a reason because it may not work.

1. Go to the [latest release](https://github.com/CCDirectLink/CCLoader/releases/latest) page of CCLoader.
2. Download the zip archive named "source code" if you are on MS Windows, or the tar.gz one if you are on macOS or GNU/Linux (again, [why tar.gz?](#why-targz-and-not-zip)). The name of the archive might change in the future, but we will keep the formats as they are now.
3. Unpack the archive directly into [the CrossCode game data directory](#what-is-the-crosscode-game-data-directory). Although it is recommended that you don't overwrite the `package.json` file - doing so may cause hard to debug issues.

   **NOTE:** there a few mods bundled together with CCLoader:

   1. `simplify` - provides functionality commonly used by mod authors, many other mods depend on it.
   2. `ccloader-version-display` - miscellaneous mod which displays CCLoader version in the bottom right corner of the main menu, near the CrossCode version.
   3. `openDevTools` - opens Chrome developer tools upon start up if you are using the SDK flavor of nw.js (might get deprecated soon).

   While `ccloader-version-display` and `openDevTools` are optional, you shouldn't delete `simplify`.

4. Open the file `package.json` in your preffered text editor. It should look something like this:

   ```json
   {
     "name": "CrossCode",
     "version": "1.0.0",
     "main": "assets/node-webkit.html",
     "chromium-args": "--ignore-gpu-blacklist --disable-direct-composition --disable-background-networking --in-process-gpu --password-store=basic",
     "window": {
       "toolbar": false,
       "icon": "favicon.png",
       "width": 1136,
       "height": 640,
       "fullscreen": false
     }
   }
   ```

   You have to replace `assets/node-webkit.html` in the line which starts with `"main":` with `ccloader/index.html`. The result should look like:

   ```json
   {
     "name": "CrossCode",
     "version": "1.0.0",
     "main": "ccloader/index.html",
     "chromium-args": "--ignore-gpu-blacklist --disable-direct-composition --disable-background-networking --in-process-gpu --password-store=basic",
     "window": {
       "toolbar": false,
       "icon": "favicon.png",
       "width": 1136,
       "height": 640,
       "fullscreen": false
     }
   }
   ```

## Where do I put mods?

The mods directory is called `mods` and is located in the `assets` directory inside of [CrossCode's game data directory](#what-is-the-crosscode-game-data-directory).

## Uninstalling CCLoader

1. Open your [CrossCode game data directory](#what-is-the-crosscode-game-data-directory).
2. Delete the directory named `ccloader`.
3. Optionally, you can delete the `mods` directory inside `assets` as well.
4. Revert the changes to made `package.json` - see the last step of [the manual installation guide](#manual-ccloader-installation-guide). Steam also has the _"Verify Integrity of Game Files"_ button, pressing it will revert the changes made to game files.

## What is the "CrossCode game data directory"?

Basically it is the directory which contains CrossCode assets, CCLoader (after its installation, of course) and the file named `package.json`. It has roughly the following layout (I excluded uninteresting files):

```
assets/                 CrossCode assets
  data/                 JSON data files
  js/                   compiled JavaScript code
  media/                sprites, sound effects, background music etc
  ...
  mods/                 this directory is created by ccloader-installer, you can put mods here
  ...
  node-webkit.html      the file which is opened when the game starts
ccloader/               this is where the installer puts everything related to CCLoader
  ...
package.json
```

It is commonly located in the CrossCode installation directory on Windows and GNU/Linux, on macOS it is located in `<CrossCode installation directory>/CrossCode.app/Contents/MacOS/Resources/app.nw`.

Here are the paths to the CrossCode installation directory if you installed it via Steam:

- **MS Windows:** `C:\Program Files\Steam\steamapps\common\CrossCode` or `C:\Program Files (x86)\Steam\steamapps\common\CrossCode`
- **macOS:** `~/Library/Application Support/Steam/steamapps/common/CrossCode` \
  path to the game data directory for convenience: `~/Library/Application Support/Steam/steamapps/common/CrossCode/CrossCode.app/Contents/MacOS/Resources/app.nw`
- **GNU/Linux and other UNIX-like OSes:** `~/.local/share/Steam/steamapps/common/CrossCode`

## Why `.tar.gz` and not `.zip`?

~~Because dmitmel is a UNIX zealot.~~

Basically because `.tar.gz` stores macOS and Linux (i.e. UNIX) file attributes, whereas `.zip` stores Windows (i.e. MS-DOS) ones. Therefore, it's more convenient (although not strictly necessary) to use `.zip` archives in the Windows world, and `.tar.gz` ones everywhere else. `.zip` is usually seen as a common archive interchange format between all OSes, although I use OS-specific archives when distributing files specifically for Windows or, say, specifically for macOS.

[Relevant StackExchange answer](https://superuser.com/a/1257441)

## What does the installer exactly do?

1. Fetches the information about the latest release of CCLoader from [CCModDB](https://github.com/CCDirectLink/CCModDB).
2. Downloads the latest release `.tar.gz` archive of CCLoader. I chose to use `.tar.gz` here instead of `.zip` or OS-specific archives because:
   1. [MS-DOS file attributes](https://en.wikipedia.org/wiki/File_attribute#DOS_and_Windows) are practically useless in our case, but the `executable` flag of UNIX file permissions may come in handy if we choose to distribute scripts or helper programs together with CCLoader.
   2. The Rust implementation of [Tar](<https://en.wikipedia.org/wiki/Tar_(computing)>) is smaller than of Zip.
   3. I can reuse the [gzip](https://en.wikipedia.org/wiki/Gzip) implementation from [zlib](https://en.wikipedia.org/wiki/Zlib) since [libcurl](https://en.wikipedia.org/wiki/CURL) already depends on it.
3. Unpacks `ccloader` and `assets/mods` subdirectories from the archive into a temporary directory inside of CrossCode's game data directory.
4. Moves `ccloader` from the temporary directory to the game data directory.
5. Creates directory `assets/mods` in the game data directory.
6. Moves mods from the temporary directory to `assets/mods` if they aren't already there.
7. Patches `package.json` as described in [the manual installation guide](#manual-ccloader-installation-guide).

## Contacts

`ccloader-installer` is developed primarily by me, [@dmitmel](https://github.com/dmitmel). You can contact me (to request features or support) either via [the bug tracker](https://github.com/CCDirectLink/ccloader-installer/issues), [the official CrossCode Discord server](https://discord.gg/crosscode) or [the CCDirectLink Discord server](https://discord.gg/3Xw69VjXfW) (the last way is the preffered one).
