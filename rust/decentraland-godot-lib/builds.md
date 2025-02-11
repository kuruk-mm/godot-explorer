
# Dependencies

Both android and iOS you need to clone ffmpeg-kit and build it.

## TODO
1. See what libraries from the kit should be compiled instead of using `--full`

## Android
TODO

## iOS
After cloning `ffmpeg-kit` ensure you have the right environment to build it. Some tips:
1. Install one-by-one the brew dependencies `autoconf automake libtool pkg-config curl git doxygen nasm cmake gcc gperf texinfo yasm bison autogen wget gettext meson ninja ragel groff gtk-doc-tools libtasn1`
2. Ensure you `bison` version in your path is > 2.4 (`bison --help`), probably you need to add the brew isntalled to PATH

The command to build it:
`bash ios.sh --full --disable-armv7 --disable-armv7s --disable-arm64-mac-catalyst --disable-arm64-simulator --disable-arm64e --disable-i386 --disable-x86-64 --disable-x86-64-mac-catalyst --disable-lib-fribidi --disable-lib-openssl --disable-lib-libass`
This only buils for arm64. `fribidi` and `libass` should be compiled but I got errors when I tried to include them.


# Build

## iOS

1. Once the build is done, you need to modify the `ios-build.sh` the line:
    - `export FFMPEG_DIR=$FFMPEG_FOLDER/prebuilt/apple-ios-arm64/ffmpeg` 
    - Replace $FFMPEG_FOLDER with your path where you clone ffmpeg_kit

2. Open godot (see the README.md on the root folder of this repo) and make an export of iOS. TeamID is required to export it.

3. Open the project generated with XCode and add the ffmpeg libraries:
    - Copy all the `.framework` folders of the $FFMPEG_FOLDER/prebuilt/bundle-apple-framework-ios to the folder generated by Godot
    - Go to the Project view, and go to File>Add file, select all the .frameworks and add them.
    - Select the app target and then the General tab
    - Scroll down to the "Framework, Libraries, and Embedded Content" section.
    - Make sure your framework is "Embed & Sign".

## Android

1. Once the build is done, you need to modify the `android-build.sh` the line:
    - `export FFMPEG_DIR=$FFMPEG_FOLDER/prebuilt/apple-ios-arm64/ffmpeg` 
    - Replace $FFMPEG_FOLDER with your path where you clone ffmpeg_kit