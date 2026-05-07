# Development
This guide details setup and development workflow for contributors.

## Development Setup

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `cargo test`
5. Submit a pull request

See [CONTRIBUTING](https://github.com/Peter-L-SVK/pro_audio_config/blob/main/CONTRIBUTING.md) file for details on branching and commit strategy.  

### Project Structure

```
pro_audio_config/
├── src/
│   ├── main.rs              # Application entry point
│   ├── lib.rs               # Library definitions and exports
│   ├── audio.rs             # Audio device detection and settings
│   ├── audio_capture.rs     # Real-time audio monitoring
│   ├── config.rs            # PipeWire/WirePlumber configuration
│   ├── config_inspector.rs  # Configuration file browser tab
│   ├── monitoring.rs        # Monitoring tab logic
│   ├── utils.rs             # Utility functions
│   ├── ui/                  # Modular UI components
│   │   ├── mod.rs           # Module declarations and re-exports
│   │   ├── app.rs           # AudioApp struct and main window
│   │   ├── audio_tab.rs     # Output/Input audio tabs
│   │   ├── advanced_tab.rs  # Advanced configuration tab
│   │   ├── components.rs    # Shared UI widgets and helpers
│   │   ├── constants.rs     # Sample rates, bit depths, buffer sizes
│   │   ├── tab_types.rs     # TabType enum definitions
│   │   ├── dialogs.rs       # Error, success, and about dialogs
│   │   ├── helpers.rs       # String cleaning utilities
│   │   └── preferences.rs   # AppPreferences load/save
├── data/
│   └── styles.css           # GTK CSS stylesheet (if needed)
├── icons/
│   ├── 16x16/
│   │   └── pro-audio-config.png
│   ├── 32x32/
│   │   └── pro-audio-config.png
│   ├── 48x48/
│   │   └── pro-audio-config.png
│   ├── 256x256/
│   │   └── pro-audio-config.png
│   └── icon.png             # Fallback/default icon
├── tests/
│   ├── integration.rs       # Main integration tests
│   ├── audio_integration.rs # Audio-specific integration tests
│   ├── ui_integration.rs    # UI integration tests
│   └── common.rs            # Shared test utilities
├── install.sh               # Installation script
├── uninstall.sh             # Uninstallation script
├── lint.sh                  # Linting and code quality script
├── Cargo.toml               # Project dependencies
├── Cargo.lock               # Dependency lock file
├── LICENSE                  # MIT License file
└── README.md                # Project documentation
```

## Tools and Configuration

### Pre-commit Hook

A pre-commit hook is provided (`pre-commit`) to ensure code style consistency by automatically formatting code and removing trailing whitespace before commits:

```sh
# In the project root
cp pre-commit .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```
This hook will:
- Remove trailing whitespace from all Rust files (`*.rs`)
- Run `cargo fmt --all` for automatic code formatting
- Add formatted files to the current commit

You are encouraged to set up this pre-commit hook after cloning the repository.

### Linting with Clippy

Rust code should also pass linter checks using [Clippy](https://github.com/rust-lang/rust-clippy).
- Clippy is run automatically in CI; configuration is located at [./github/workflows/rust-ci.yaml](./github/workflows/rust-ci.yaml).
- You can manually run Clippy via:

  ```bash
  cargo clippy
  ```
  
- Please ensure no errors are introduced by your changes (warning may occure since linting is not perfect).

## Testing
Ensure all tests pass before submitting changes:
```bash
cargo test --lib
cargo test --tests
```
## Building from Source

### Development Build
```bash
git clone https://github.com/Peter-L-SVK/pro_audio_config
cd pro_audio_config
cargo build # -- feature flag
```

### Release Build
```bash
cargo build --release # -- feature flag
```

### Debugging

```bash
# in ./target/release or ./target/debug run:
./pro_audio_config
```
**Tools**:
```sh
journalctl
pw-cli
wireplumber
```
### Running Tests
```bash
cargo test
```
