# Aevio Watch companion

This is an independent watchOS companion for the local Aevio desktop app. It
uses the authenticated LAN bridge exposed by the Mac app, so time data remains
local and no Aevio cloud account is required.

## Pairing

The desktop command `watch_pairing` returns the persistent pairing token. Enter
the Mac hostname or local IP, port `48731`, and that token in the Watch app.
The token is stored in the desktop app-data directory with owner-only
permissions on Unix platforms.

## Building

The repository contains an XcodeGen project specification. On a Mac with the
full Xcode toolchain and an Apple Developer team configured:

```sh
xcodegen generate
open AevioWatch.xcodeproj
```

The Watch app currently supports listing timers and starting/pausing them. The
desktop bridge also supports the focus-session endpoint for the next UI slice.
