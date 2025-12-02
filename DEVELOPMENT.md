# Development Setup
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `cargo test`
5. Submit a pull request

See [CONTRIBUTING](https://github.com/Peter-L-SVK/pro_audio_config/blob/main/CONTRIBUTING.md) file for details on branching and commit strategy.  

## Project Structure

```
pro_audio_config/
├── src/
│   ├── main.rs          # Application entry point
│   ├── lib.rs           # Library definitions and exports
│   ├── audio.rs         # Audio device detection and settings
│   ├── config.rs        # PipeWire/WirePlumber configuration
│   ├── ui.rs            # GTK user interface with tabbed interface
│   └── utils.rs         # Utility functions
├── tests/
│   ├── integration.rs   # Main integration tests
│   ├── audio_integration.rs # Audio-specific integration tests
│   ├── ui_integration.rs    # UI integration tests
│   └── common.rs        # Test utilities
├── icons/               # Application icons (multiple sizes)
├── install.sh           # Installation script
├── uninstall.sh         # Uninstallation script
└── Cargo.toml           # Project dependencies
```


# Testing
Ensure all tests pass before submitting changes:
```bash
cargo test --lib
cargo test --tests
```
# Building from Source

## Development Build
```bash
git clone https://github.com/Peter-L-SVK/pro-audio-config
cd pro_audio_config
cargo build
```

## Release Build
```bash
cargo build --release
```

## Debugging

```bash
# in ./target/release or ./target/debug
./pro_audio_config
```
**Tools**:
```sh
journalctl
pw-cli
wireplumber
```
## Running Tests
```bash
cargo test
```
