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

On the 3DS, start NTR-HR. In the app, enter the console IP, select the Mac's LAN
address as Viewer IP, leave Viewer Port at `8001`, and click Connect.

## Defaults

- Control port: `8000`
- Viewer port: `8001`
- JPEG quality: `75`
- Bandwidth: `16 Mbps`
- Top-screen priority factor: `2`

