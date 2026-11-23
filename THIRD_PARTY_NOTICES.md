# Third-party notices

Mandate includes the open-source software listed below. Each component is used
under its own licence; those licences, and the copyright lines quoted here, are
the work of their authors. This file lists what ships in the game's build (the
WebAssembly engine and the web app). It does not list tools that only run on a
developer's machine or in CI.

*Groundwork, not legal advice.* This list was generated from the lock files and
the packages' own metadata on 2026-11-23 and should be regenerated before a
release (see "Regenerating" at the end). A lawyer should check it before any
commercial release.

Mandate's own code, text and art are not covered by this file. See
`docs/design/LEGAL_GROUNDWORK.md` for the status of the project's own licence.

## Summary

- No copyleft licence (GPL, LGPL, AGPL, MPL, EPL, CC-BY-SA) appears in anything
  that ships.
- Every component is permissive: MIT, Apache-2.0, BSD-2-Clause, 0BSD, Zlib,
  Unlicense, CC0-1.0, MIT-0, and Unicode-3.0 (the Unicode data licence, which
  asks that its notice be kept).
- Several crates offer a choice ("MIT OR Apache-2.0"); Mandate uses them under
  either. Apache-2.0 components require that their licence text and any NOTICE
  file travel with redistributed copies; the full texts are in each package's
  source and are linked below.
- Fonts: none are shipped. The game and the site use the visitor's own system
  fonts (`system-ui`, Georgia).
- Art: all pixel art is made by Mandate's own tool (`tools/pixelkit`) from
  parts drawn for the game; no third-party art, icons, sounds or fonts are
  included (DEC-084 D4). The landing page's screenshots are screenshots of the
  game.

## Rust crates (compiled into the engine WebAssembly)


| Crate | Version | Licence | Copyright notice found in the package |
|---|---|---|---|
| [adler2](https://github.com/oyvindln/adler2) | 2.0.1 | 0BSD OR MIT OR Apache-2.0 | Copyright (C) Jonas Schievink <jonasschievink@gmail.com> |
| [arrayvec](https://github.com/bluss/arrayvec) | 0.7.8 | MIT OR Apache-2.0 | Copyright (c) Ulrik Sverdrup "bluss" 2015-2023 |
| [blake3](https://github.com/BLAKE3-team/BLAKE3) | 1.8.7 | CC0-1.0 OR Apache-2.0 OR Apache-2.0 WITH LLVM-exception | Copyright 2019 Jack O'Connor and Samuel Neves |
| [bumpalo](https://github.com/fitzgen/bumpalo) | 3.20.4 | MIT OR Apache-2.0 | Copyright (c) 2019 Nick Fitzgerald |
| [cfg-if](https://github.com/rust-lang/cfg-if) | 1.0.5 | MIT OR Apache-2.0 | Copyright (c) 2014 Alex Crichton |
| [console_error_panic_hook](https://github.com/rustwasm/console_error_panic_hook) | 0.1.7 | Apache-2.0/MIT | Copyright (c) 2018 Nick Fitzgerald |
| [constant_time_eq](https://github.com/cesarb/constant_time_eq) | 0.4.2 | CC0-1.0 OR MIT-0 OR Apache-2.0 | see package |
| [dyn-clone](https://github.com/dtolnay/dyn-clone) | 1.0.20 | MIT OR Apache-2.0 | see package |
| [futures-core](https://github.com/rust-lang/futures-rs) | 0.3.34 | MIT OR Apache-2.0 | Copyright (c) 2016 Alex Crichton; Copyright (c) 2017 The Tokio Authors |
| [futures-task](https://github.com/rust-lang/futures-rs) | 0.3.34 | MIT OR Apache-2.0 | Copyright (c) 2016 Alex Crichton; Copyright (c) 2017 The Tokio Authors |
| [futures-util](https://github.com/rust-lang/futures-rs) | 0.3.34 | MIT OR Apache-2.0 | Copyright (c) 2016 Alex Crichton; Copyright (c) 2017 The Tokio Authors |
| [itoa](https://github.com/dtolnay/itoa) | 1.0.19 | MIT OR Apache-2.0 | see package |
| [js-sys](https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/js-sys) | 0.3.107 | MIT OR Apache-2.0 | Copyright (c) 2014 Alex Crichton |
| [libm](https://github.com/rust-lang/compiler-builtins) | 0.2.16 | MIT | Copyright (c) 2018 Jorge Aparicio; Copyright © 1993,2004 Sun Microsystems or |
| [memchr](https://github.com/BurntSushi/memchr) | 2.8.3 | Unlicense OR MIT | Copyright (c) 2015 Andrew Gallant |
| [miniz_oxide](https://github.com/Frommi/miniz_oxide/tree/master/miniz_oxide) | 0.9.1 | MIT OR Zlib OR Apache-2.0 | Copyright (c) 2017 Frommi; Copyright (c) 2017-2024 oyvindln |
| [pin-project-lite](https://github.com/taiki-e/pin-project-lite) | 0.2.17 | Apache-2.0 OR MIT | see package |
| [ppv-lite86](https://github.com/cryptocorrosion/cryptocorrosion) | 0.2.21 | MIT OR Apache-2.0 | Copyright (c) 2019 The CryptoCorrosion Contributors; Copyright 2019 The CryptoCorrosion Contributors |
| [proc-macro2](https://github.com/dtolnay/proc-macro2) | 1.0.107 | MIT OR Apache-2.0 | see package |
| [quote](https://github.com/dtolnay/quote) | 1.0.47 | MIT OR Apache-2.0 | see package |
| [rand_chacha](https://github.com/rust-random/rand) | 0.10.0 | MIT OR Apache-2.0 | Copyright (c) 2014 The Rust Project Developers; Copyright 2018 Developers of the Rand project |
| [rand_core](https://github.com/rust-random/rand_core) | 0.10.1 | MIT OR Apache-2.0 | Copyright (c) 2018-2026 The Rand Project Developers |
| [ref-cast](https://github.com/dtolnay/ref-cast) | 1.0.27 | MIT OR Apache-2.0 | see package |
| [ref-cast-impl](https://github.com/dtolnay/ref-cast) | 1.0.27 | MIT OR Apache-2.0 | see package |
| [schemars](https://github.com/GREsau/schemars) | 1.2.3 | MIT | Copyright (c) 2019 Graham Esau |
| [schemars_derive](https://github.com/GREsau/schemars) | 1.2.3 | MIT | Copyright (c) 2019 Graham Esau |
| [serde](https://github.com/serde-rs/serde) | 1.0.229 | MIT OR Apache-2.0 | see package |
| [serde_core](https://github.com/serde-rs/serde) | 1.0.229 | MIT OR Apache-2.0 | see package |
| [serde_derive](https://github.com/serde-rs/serde) | 1.0.229 | MIT OR Apache-2.0 | see package |
| [serde_derive_internals](https://github.com/serde-rs/serde) | 0.30.0 | MIT OR Apache-2.0 | see package |
| [serde_json](https://github.com/serde-rs/json) | 1.0.154 | MIT OR Apache-2.0 | see package |
| [slab](https://github.com/tokio-rs/slab) | 0.4.12 | MIT | Copyright (c) 2019 Carl Lerche |
| [syn](https://github.com/dtolnay/syn) | 3.0.10 | MIT OR Apache-2.0 | see package |
| [syn](https://github.com/dtolnay/syn) | 2.0.119 | MIT OR Apache-2.0 | see package |
| [termcolor](https://github.com/BurntSushi/termcolor) | 1.4.1 | Unlicense OR MIT | Copyright (c) 2015 Andrew Gallant |
| [thiserror](https://github.com/dtolnay/thiserror) | 2.0.21 | MIT OR Apache-2.0 | see package |
| [thiserror-impl](https://github.com/dtolnay/thiserror) | 2.0.21 | MIT OR Apache-2.0 | see package |
| [ts-rs](https://github.com/Aleph-Alpha/ts-rs) | 12.0.1 | MIT | see package |
| [ts-rs-macros](https://github.com/Aleph-Alpha/ts-rs) | 12.0.1 | MIT | see package |
| [unicode-ident](https://github.com/dtolnay/unicode-ident) | 1.0.27 | (MIT OR Apache-2.0) AND Unicode-3.0 | Copyright © 1991-2023 Unicode, Inc. |
| [wasm-bindgen](https://github.com/wasm-bindgen/wasm-bindgen) | 0.2.130 | MIT OR Apache-2.0 | Copyright (c) 2014 Alex Crichton |
| [wasm-bindgen-macro](https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/macro) | 0.2.130 | MIT OR Apache-2.0 | Copyright (c) 2014 Alex Crichton |
| [wasm-bindgen-macro-support](https://github.com/wasm-bindgen/wasm-bindgen/tree/main/crates/macro-support) | 0.2.130 | MIT OR Apache-2.0 | Copyright (c) 2014 Alex Crichton |
| [wasm-bindgen-shared](https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/shared) | 0.2.130 | MIT OR Apache-2.0 | Copyright (c) 2014 Alex Crichton |
| [zerocopy](https://github.com/google/zerocopy) | 0.8.63 | BSD-2-Clause OR Apache-2.0 OR MIT | Copyright 2019 The Fuchsia Authors.; Copyright 2023 The Fuchsia Authors |
| [zmij](https://github.com/dtolnay/zmij) | 1.0.23 | MIT | see package |

The Rust standard library and the compiler's runtime support are MIT OR
Apache-2.0. The full text of each crate's licence is in its source package on
crates.io (`https://crates.io/crates/<name>/<version>`, "Licenses" in the
repository).

## npm packages (bundled into the web app and its service worker)

Production dependencies and the runtime pieces the build bundles. Build tools
(Vite, TypeScript, ESLint, Playwright, Vitest and the like) are not shipped.


| Package | Version | Licence | Copyright notice |
|---|---|---|---|
| [@fluent/bundle](https://github.com/projectfluent/fluent.js) | 0.19.1 | Apache-2.0 | see package |
| [dexie](https://github.com/dexie/Dexie.js) | 4.4.7 | Apache-2.0 | see package |
| [react](https://github.com/react/react) | 19.3.1 | MIT | Copyright (c) Meta Platforms, Inc. and affiliates. |
| [react-dom](https://github.com/react/react) | 19.3.1 | MIT | Copyright (c) Meta Platforms, Inc. and affiliates. |
| [scheduler](https://github.com/react/react) | 0.28.0 | MIT | Copyright (c) Meta Platforms, Inc. and affiliates. |
| [workbox-core](https://github.com/googlechrome/workbox) | 7.4.1 | MIT | Copyright 2018 Google LLC |
| [workbox-precaching](https://github.com/googlechrome/workbox) | 7.4.1 | MIT | Copyright 2018 Google LLC |
| [workbox-routing](https://github.com/googlechrome/workbox) | 7.4.1 | MIT | Copyright 2018 Google LLC |
| [workbox-strategies](https://github.com/googlechrome/workbox) | 7.4.1 | MIT | Copyright 2018 Google LLC |
| [workbox-window](https://github.com/googlechrome/workbox) | 7.4.1 | MIT | Copyright 2018 Google LLC |
| [zustand](https://github.com/pmndrs/zustand) | 5.1.0 | MIT | Copyright (c) 2019 Paul Henschel |

`workbox-*` are bundled into the service worker (`play/sw.js`) by
`vite-plugin-pwa`; `workbox-window` is bundled into the app.

## Fonts

None shipped.

## Art, sound and other assets

- Pixel art, icons and the logo mark: made for Mandate, procedurally composed
  by `tools/pixelkit` from parts drawn for the game. Not third-party.
- Landing-page images: screenshots of the game, and an Open Graph card made
  from them.
- Sound: none.
- Player-facing text: written for the game.

## Regenerating

```text
cargo metadata --format-version 1 --filter-platform wasm32-unknown-unknown --locked
cd web && npm ci && npm run build   # then read the sources in dist/play/assets/*.map
```

`cargo about` or `cargo deny` (Rust) and `license-checker-rseidelsohn` (npm)
can produce the same lists with full licence texts attached; adopt one of them
before a store release.
