# Contributing to Pro Audio Config

Thank you for your interest in contributing to Pro Audio Config! This project aims to make professional audio configuration accessible to all Linux users, and we welcome contributions from the community.

## Development Workflow

### Branching Strategy

We use a multi-branch workflow to keep development organized and stable:

- **main**: Always stable and production-ready. Only thoroughly tested changes should be merged here.
- **stage**: Used for integrating, testing, and reviewing new features before they reach `main`. Most pull requests should target this branch.
- **feat/*** (feature branches): Create a new branch from `stage` for each new feature or bugfix. Use clear, descriptive names (e.g., `feat/pipewire-support` or `fix/device-detection`).

### How to Contribute

1. **Update your local repository:**
   ```bash
   git checkout stage
   git pull
   ```

2. **Create a new feature branch:**
   ```bash
   git checkout -b feat/describe-feature
   ```

3. **Work on your feature or fix.**  
   Commit changes as needed.

4. **Push your branch and open a Pull Request to `stage`:**
   ```bash
   git push -u origin feat/describe-feature
   ```
   Then open a Pull Request targeting the `stage` branch.

5. **After review and testing, your changes will be merged into `stage`.**  
   Periodically, stable changes from `stage` will be merged into `main`.

6. **Delete feature branches after merging** to keep the repository tidy.

### Branch Naming
- Use one of the following prefixes for your branch names:  
  - `feat/describe-feature` (new functionality)  
  - `fix/describe-fix` (bug corrections)  
  - `docs/topic` (documentation updates)
- Please keep branch names short and descriptive.

## Commit Message Standards

```
type(scope): brief description [issue #]

[Detailed explanation if needed]
```

**Types:**
- `feat`: New functionality
- `fix`: Bug fixes
- `docs`: Documentation changes
- `style`: Code formatting
- `refactor`: Code restructuring
- `test`: Test-related changes
- `audio`: Audio-specific changes
- `ui`: User interface changes

**Examples:**
```
feat(pipewire): Add high-resolution sample rate support [issue #15]

Implemented 384kHz sample rate configuration for PipeWire.
Includes validation for hardware capability detection.
```

```
fix(device-detection): Handle USB audio device hot-plugging [issue #23]

- Improved device enumeration after hot-plug events
- Added fallback detection for PulseAudio systems
- Fixed race condition in device scanning
```

## Development Setup

### Prerequisites
- Rust 1.85+
- Cargo
- GTK3 development libraries
- PipeWire development headers (optional)

### Getting Started

1. **Fork the repository**
   ```bash
   git clone https://github.com/Peter-L-SVK/TermTalk
   cd pro_audio_config
   ```

2. **Build the project**
   ```bash
   cargo build
   ```

3. **Run tests**
   ```bash
   cargo test
   ```

4. **Test the application**
   ```bash
   cargo run
   ```

## Code Standards

### Rust Code Style
- Follow Rustfmt formatting
- Use clippy for linting
- Document public APIs with /// comments
- Write meaningful commit messages

### GTK/UI Guidelines
- Maintain consistent UI patterns
- Follow GNOME HIG where applicable
- Ensure accessibility considerations
- Test on different GTK themes

### Audio Code Standards
- Handle both PipeWire and PulseAudio gracefully
- Provide clear error messages for audio operations
- Include fallback mechanisms for different systems
- Test across multiple audio hardware configurations

## Testing Requirements

### Running Tests
```bash
# Run all tests
cargo test

# Run specific test suites
cargo test --lib          # Library tests
cargo test --tests        # Integration tests
cargo test audio          # Audio module tests
```

### Test Coverage
- Unit tests for core logic
- Integration tests for audio operations
- UI tests for critical user flows
- Cross-distribution testing encouraged

### Audio System Testing
New features require testing on:
- PipeWire systems (primary)
- PulseAudio systems (fallback)
- Different audio hardware (USB, PCI, HDMI)
- Various Linux distributions

## Versioning Policy

### Release Tagging
- Use semantic versioning (MAJOR.MINOR.PATCH) for GitHub releases
- Follow conventional commit messages for automated versioning
- Major versions (2.0.0) for breaking changes
- Minor versions (1.1.0) for new features
- Patch versions (1.0.1) for bug fixes

## Ways to Contribute

### 🐛 Reporting Bugs
- Check existing issues to avoid duplicates
- Use the bug report template
- Include system information (distribution, audio hardware, PipeWire version)
- Provide steps to reproduce the issue

### 💡 Suggesting Features
- Check the roadmap in README.md first
- Explain the use case and benefits
- Consider if it aligns with the project's goals

### 🔧 Code Contributions
- Fix bugs
- Implement new features
- Improve documentation
- Add tests
- Optimize performance

### 📚 Documentation
- Improve README.md
- Add code comments
- Create tutorials or guides
- Translate documentation

## Pull Request Process

1. **Create a feature branch** from `stage`
2. **Make your changes** with proper commit messages
3. **Write or update tests** as needed
4. **Update documentation** for new features
5. **Ensure all tests pass**
6. **Submit pull request** to `stage` branch with:
   - Descriptive title and description
   - Reference to related issues
   - Screenshots for UI changes

## Areas Needing Help

### High Priority
- Additional audio backend support (JACK)
- Package creation (RPM, DEB, Flatpak)
- Improved error handling and user feedback

### Medium Priority
- Internationalization (i18n)
- Preset configurations
- Advanced audio routing UI
- Setting all available devices in one session

### Documentation
- User guides for different audio setups
- Troubleshooting common issues
- Developer documentation

## Communication

### Discussion Channels
- GitHub Issues for bugs and features
- Pull Requests for code review
- Project discussions for ideas

### Code of Conduct
- Be respectful and inclusive
- Provide constructive feedback
- Help others learn and contribute

## Recognition

Contributors will be:
- Listed in the README.md (if desired)
- Credited in release notes
- Acknowledged in the about dialog

---

Thank you for helping make Linux audio configuration better for everyone! 🎵

*"Stop fighting config files. Start making great audio."*
