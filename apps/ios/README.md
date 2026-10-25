# Aevio for iPhone

The iPhone companion connects to the running Aevio desktop app over the
authenticated local-network bridge on port `48731`. It reads and mutates the
same Rust-owned timer state as the desktop and watchOS clients; it does not
keep a second timer database.

## Build with Xcode

With Xcode 27.2 or newer installed:

```sh
xcodegen generate
open Aevio.xcodeproj
```

The Debug project is intentionally unsigned for simulator builds. A physical
device or distribution build needs the team's normal Apple signing settings.
