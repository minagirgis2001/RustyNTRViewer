# RustyNTRViewer

A memory-safe, cross-platform NTR/NTR-HR viewer written in Rust.

The current milestone implements the NTR control channel and classic JPEG-over-UDP
streaming end to end. Reliable, delta, and lossless modes are represented in the
public protocol API and automatically fall back to classic JPEG while their pure
Rust codecs are completed.

## Run

```sh
cargo run -p rusty-ntr-viewer --release
```

Pass `--connect` after `--` to connect immediately using the saved/default
addresses, which is useful for smoke tests:

```sh
cargo run -p rusty-ntr-viewer --release -- --connect
```

On the 3DS, start NTR-HR. In the app, enter the console IP, select the Mac's LAN
address as Viewer IP, leave Viewer Port at `8001`, and click Connect.

## Defaults

- Control port: `8000`
- Viewer port: `8001`
- JPEG quality: `75`
- Bandwidth: `16 Mbps`
- Top-screen priority factor: `2`

## Continuous integration and releases

Pull requests and pushes to `main` run formatting, Clippy, and workspace tests on
Linux, macOS, and Windows. CI also enforces the dependency license allow-list
and verifies that the generated third-party license bundle matches `Cargo.lock`.
The protected `main` branch uses the aggregate `Required` job as its stable
required-check target.

Pushing a semantic-version tag builds and publishes a GitHub release:

```sh
git tag v0.1.0
git push origin v0.1.0
```

Release artifacts include a universal macOS `.dmg`, a Windows x86_64 `.zip`
containing the `.exe`, and Linux x86_64 `.tar.gz` and `.deb` packages. The
release workflow can also be run manually to build artifacts without publishing
a GitHub release. Pull requests that affect application or packaging code build
the same packages without publishing them, catching release-only failures before
merge.

## Third-party licenses

Release packages contain `THIRD_PARTY_NOTICES.md` and the complete generated
`THIRD_PARTY_LICENSES.html` attribution bundle. After changing dependencies,
install the pinned generator and refresh the bundle:

```sh
cargo install cargo-about --version 0.9.1 --locked --features cli
cargo fetch --locked
.github/scripts/generate-licenses.sh
```

The accepted license policy and supported release targets are declared in
`about.toml` and `deny.toml`.
