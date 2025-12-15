# Pro Audio Config <div style="text-align: right">[![Rust CI](https://github.com/Peter-L-SVK/pro_audio_config/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/Peter-L-SVK/pro_audio_config/actions/workflows/rust-ci.yml)</div>

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) 
[![Github All Releases](https://img.shields.io/github/downloads/Peter-L-SVK/pro_audio_config/total.svg)](https://github.com/Peter-L-SVK/pro_audio_config) <a href="https://buymeacoffee.com/leukanic.peter"><img src="https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png" height="20px"></a>

A professional audio configuration tool for Linux systems that provides a simple graphical interface to manage PipeWire and ALSA audio settings. Finally, an easy way to configure sample rates, bit depths, and buffer sizes without digging through config files.

## Why This Exists

After 10 years of using Fedora and other Linux distributions, I realized we're still missing a fundamental tool: a simple, graphical way to configure professional audio settings. In 2025, manually editing configuration files shouldn't be the only option for adjusting basic audio parameters. If you wish to support me for future updates with some symbolical donation, BuyMeACoffe button.

![Running the program](scrshots/scr1.png)  
*(Program running)*

## Features

All current features and running on different OSes are showcased in the  [Gallery](https://github.com/Peter-L-SVK/pro_audio_config/blob/main/scrshots/README.md)

### 🎚️ **Real-time Audio Monitoring Tab**
- **Live Audio Level Meters**: Visual monitoring of stereo channel output levels with real-time PipeWire integration
- **Color-coded Visual Feedback**: Safe (green), warning (yellow), and clipping (red) zones with blinking indicators
- **Real System Audio Monitoring**: Taps into PipeWire monitor sources for accurate system-wide audio level measurement
- **Smooth Meter Animation**: 10 FPS updates with proper threading for responsive UI performance
- **DB Scale Display**: Accurate decibel readings with peak and RMS calculations
- **Auto-detection Fallback**: Intelligent fallback to simulation mode when PipeWire isn't available
- **Professional Metering**: Studio-grade level monitoring with proper threshold matching industry standards

### 🔧 **Device and Configuration Management**
- **Device Detection**: Automatically detects and displays current audio devices (PipeWire, ALSA, PulseAudio)
- **Sample Rate Configuration**: Support for rates from 44.1 kHz to 384 kHz
- **Bit Depth Settings**: 16-bit, 24-bit, and 32-bit audio formats
- **Buffer Size Control**: Adjustable buffer sizes from 128 to 8192 samples
- **Professional Audio Support**: Optimized for studio and high-quality audio applications
- **Graphical Interface**: Clean, intuitive GTK-based user interface
- **No Terminal Required**: Graphical interface that anyone can use
- **Privilege Escalation**: Secure privilege escalation for system-level audio configuration

### 🗂️ **Multi-Tab Interface**
- **Separate Input/Output Configuration**: Dedicated tabs for playback (output) and recording (input) devices
- **Real-time Monitoring Tab**: Dedicated monitoring interface with live audio meters
- **Independent Settings Management**: Configure input and output devices with different sample rates, bit depths, and buffer sizes
- **Clear Visual Separation**: Intuitive tabbed interface that clearly distinguishes between recording, playback, and monitoring functions

### ⚡ **Advanced Tab Features**
- **Configuration Modes**: Switch between Global System Settings and Exclusive Mode
- **Professional Audio Settings**: Fine-tune buffer size ranges, thread priorities, and memory locking
- **Exclusive Mode**: Direct hardware access for single applications (similar to ASIO/WASAPI exclusive mode)
- **Latency Calculation**: Real-time latency display based on buffer size and sample rate
- **Hardware Optimization**: Memory locking, device suspend prevention, and clock source selection
- **Resampling Control**: Adjust resampler quality or disable automatic resampling entirely
- **Channel Management**: Option to disable automatic channel remixing for professional workflows

### 🔍 **Configuration Scope**
- **User-Specific Configuration**: Apply settings only for the current user (default)
- **System-Wide Configuration**: Apply settings for all users on the system (requires admin privileges)
- **Flexible Deployment**: Choose between user-specific or system-wide configuration based on your needs

### 🛠️ **Advanced Configuration**
- **Dedicated WirePlumber Configs**: Separate configuration generators for input vs output devices
- **Independent Signal Handlers**: Each tab manages its own settings and apply operations
- **Comprehensive Testing**: Extensive test suite covering new input/output separation features
- **Advanced configuration options** ready for professional studio workflows
- **Exclusive mode support** for applications requiring ASIO-like direct hardware access
- **Real-time performance tuning** with thread priority and memory management

## Supported Audio Systems

- PipeWire (primary)
- ALSA (fallback)
- PulseAudio (legacy support)

## Future Proof Design

This tool is designed to grow with PipeWire and Linux audio:

- **Extended buffer support** ready for future PipeWire versions
- **Modular architecture** for easy addition of new audio backends
- **Hardware-agnostic** - works with any ALSA-compatible device
- **Standards-based** using industry-standard audio formats
- **Extensible UI** that can accommodate new settings and features

As audio companies improve their Linux driver support and PipeWire continues to evolve, this tool will be ready to leverage those advancements immediately.

## Roadmap

- [x] **Real-time Audio Monitoring** - Live level meters with PipeWire integration
- [x] Setting all available devices in one session
- [x] Separate input/output configuration tabs
- [x] User vs system-wide configuration scope
- [ ] Package manager support (RPM/DEB/Flatpak)
- [ ] Additional audio backends (JACK)
- [ ] Preset configurations
- [x] Advanced audio routing

## Installation

### Setup

1. Install rust and cargo:
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh # select 1

# sh/bash/zsh/dash/ash/pdksh etc
. "$HOME/.cargo/env" # for fish and nushell see output of rust install
```

2. Install dependencies for app building:
```bash
# Fedora/CentOS Stream/RHEL/Rocky
sudo dnf install  pipewire-devel clang llvm-dev gtk3-devel

# Arch/EndeavourOS/Manjaro/arch-based
sudo pacman -S pipewire clang llvm gtk3 # pkg-config add in Manjaro

# Ubuntu/Debian/Mint/PopOS/ZorinOS
sudo apt install libpipewire-0.3-dev clang llvm-dev libgtk-3-dev
```

### Quick Install
```bash
git clone https://github.com/Peter-L-SVK/pro_audio_config # or download as zip manually and unzip
cd pro_audio_config
./install.sh
```

### From Source

1. Clone the repository:
```bash
git clone https://github.com/Peter-L-SVK/pro_audio_config
cd pro_audio_config
```

2. Build and install:
```bash
cargo build --release --features pipewire-monitoring
sudo cp target/release/pro_audio_config /usr/local/bin/
```

### Uninstall
```bash
./uninstall.sh
```

## Usage

### Graphical Interface
Run the application from your application menu or terminal:
```bash
pro-audio-config
```

### Input/Output Tabs
- **Output Tab**: Configure playback devices (speakers, headphones, HDMI outputs)
- **Input Tab**: Configure recording devices (microphones, audio interfaces, line inputs)

### Advanced Tab
The Advanced Tab provides professional-grade audio configuration options:

#### Configuration Modes
- **Global System Settings**: Configure audio settings that apply to ALL applications system-wide
- **Exclusive Mode**: Provide direct hardware access to a SINGLE application, bypassing mixing and processing

#### Professional Settings
- **Buffer Size Range**: Set minimum and maximum buffer sizes for dynamic adjustment
- **Real-time Thread Priority**: Control CPU scheduling priority for audio threads
- **Memory Locking**: Lock audio buffers in RAM to prevent swapping and reduce latency
- **Device Suspend Prevention**: Keep audio devices active even when idle
- **Resampler Quality**: Adjust the quality/performance tradeoff for sample rate conversion
- **Clock Source Selection**: Choose between system clock, monotonic, or real-time clock sources

#### Exclusive Mode Features
- **Application Targeting**: Specify which application should receive exclusive audio access
- **Direct Hardware Access**: Bypass audio mixing for lowest possible latency
- **Real-time Latency Display**: See calculated latency based on your buffer size and sample rate
- **Device Capability Checking**: Automatic validation of device compatibility with exclusive mode

#### Using Exclusive Mode
1. Select "Exclusive Mode" from the Configuration Mode dropdown
2. Enter the application name and process name (e.g., "Reaper" and "reaper")
3. Select your audio device and desired buffer size/sample rate
4. Click "Apply Exclusive Mode Settings"
5. To return to standard shared mode, click "Disable Exclusive Mode"

Each tab maintains independent settings, allowing you to optimize input and output devices separately for your specific use case.

### Configuration Scope
- **User-Specific (Default)**: Settings apply only to your user account, stored in `~/.config/pipewire/`
- **System-Wide**: Settings apply to all users, stored in `/etc/pipewire/` (requires authentication)
- **Toggle Option**: Use the "Apply system-wide" checkbox to switch between user and system configuration

## Configuration

### Sample Rates
- 44.1 kHz - CD Quality
- 48 kHz - Standard Audio
- 96 kHz - High Resolution
- 192 kHz - Studio Quality
- 384 kHz - Ultra High Resolution

### Bit Depths
- 16 bit - CD Quality
- 24 bit - High Resolution
- 32 bit - Studio Quality

### Buffer Sizes
- 128 samples (2.7ms @48kHz)
- 256 samples (5.3ms @48kHz)
- 512 samples (10.7ms @48kHz)
- 1024 samples (21.3ms @48kHz)
- 2048 samples (42.7ms @48kHz)
- 4096 samples (85.3ms @48kHz) 
- 8192 samples (170.7ms @48kHz)

## Technical Details

### Architecture
- Frontend: GTK3 with Rust bindings
- Audio Backend: PipeWire with ALSA fallback
- Configuration: WirePlumber for session management
- Privilege Escalation: pkexec for secure root access
- Multi-threaded UI: Non-blocking device detection and configuration

### Configuration Scope Implementation
- **User Configuration**: Uses `~/.config/pipewire/pipewire.conf.d/` directory
- **System Configuration**: Uses `/etc/pipewire/pipewire.conf.d/` directory
- **Preference Persistence**: User preferences saved in `~/.config/proaudioconfig/preferences.toml`
- **Authentication**: Uses system's policy kit for secure privilege escalation

### Audio Format Mapping
- 16-bit: S16LE
- 24-bit: S24LE
- 32-bit: S32LE

### Configuration Approaches
- **Primary**: PipeWire config fragments (highest priority)
- **Fallback**: WirePlumber JSON configuration
- **Emergency**: Direct modification of main pipewire.conf


## Development and testing
See [DEVELOPMENT](https://github.com/Peter-L-SVK/pro_audio_config/blob/main/DEVELOPMENT.md) file for details.  


## Contributing

Contributions are welcome! Please feel free to submit pull requests, report bugs, or suggest new features.  

See [CONTRIBUTING](https://github.com/Peter-L-SVK/pro_audio_config/blob/main/CONTRIBUTING.md) file for details.  

For contact please see my email in profile info or use GitHub's built-in communication tools.

Please open an issue or pull request for any:  

- Bug fixes
- Feature suggestions
- Documentation improvements


## Support

For support and questions:
- Open an issue on GitHub
- Check the documentation
- Review existing issues for similar problems


## Requirements

### System Dependencies
- PipeWire (recommended) or PulseAudio
- pipewire-devel redhat based/ libpipewire-0.3-dev for debian/ubuntu based (for building the app)
- clang and llvm-dev (for building the app)
- WirePlumber
- ALSA utilities
- pkexec (for privilege escalation)

### Rust Toolchain
- Rust 1.85.0 or newer
- Cargo

### GTK Development Libraries
- gtk3-devel (or equivalent for your distribution, on debian+ubuntu based: libgtk-3-dev)

### Tested on
- Fedora 42 Cinnamon (fully works)
- Fedora 43 Jam with KDE Plasma 6.5 (fully works)
- Mint MATE 22.2 Zara (fully works)
- Ubuntu MATE 24.04.03 LTS (fully works)
- Pop!_OS 24.04 LTS Cosmic desktop (live monitor isn't showing activity due to Cosmic UI differency, but configs and info showing works)
- Manjaro with Gnome 49 (fully works)
- EndeavourOS Ganymede with KDE Plasma 6.5 (fully works)

**Note:** Since this works on more conservative Ubuntu LTS based systems, newer non LTS Ubuntu OSes will defintelly work...


## Acknowledgments

- PipeWire developers for the modern audio system
- GTK team for the GUI toolkit
- Rust community for excellent libraries and tools


## License

This project is licensed under the [MIT License](LICENSE) - see the LICENSE file for details.

---
**Stop fighting config files. Start making great audio.**
