# Third-party notices

Protocol behavior was studied from [xzn/ntrviewer-hr](https://github.com/xzn/ntrviewer-hr),
released under the MIT License (copyright 2025), and from its KCP implementation,
released under the MIT License (copyright 2017 Lin Wei). No source from those
projects is compiled into this application.

JPEG decoding and the desktop interface are provided by the Rust dependency
ecosystem under their published licenses. Complete dependency copyright and
license terms are included in `THIRD_PARTY_LICENSES.html`, generated from the
locked dependency graph with `cargo-about`.

The generated file covers the Linux x86_64, macOS arm64/x86_64, and Windows
x86_64 release targets. CI rejects dependencies outside the repository's
permissive license policy and rejects a stale generated license file.
