# AUR AACS KeyDB Daily Updater

This is an automated tool for maintaining the [`aacs-keydb-daily`](https://aur.archlinux.org/packages/aacs-keydb-daily) package on AUR.

## Features

1. **Automatic archiving**: Archive `http://fvonline-db.bplaced.net/export/keydb_eng.zip` on web.archive.org
2. **SHA256 verification**: Compare current AUR package SHA256 with archived file SHA256
3. **Automatic generation**: If differences are detected, automatically generate AUR package files including:
   - Update version number (based on archive timestamp: YYYYMMDDhhmmss)
   - Update SHA256 hash
   - Replace original link with web.archive.org archive link
   - Generate new .SRCINFO file
   - Commit and push changes to AUR repository
4. **Structured logging**: Uses the `tracing` library for hierarchical, structured logging that can be filtered by level and output to stdout, files, or syslog

## Configuration

### Environment Variables

Create a `.env` file (optional):

```env
# SSH Key Path for AUR repository access
# Default: ~/.ssh/id_ed25519
SSH_KEY_PATH=~/.ssh/id_ed25519_aur

# Log level configuration (optional)
# Default: info
# Options: trace, debug, info, warn, error
RUST_LOG=info
```

### SSH Key

Ensure you have SSH access to AUR:

- Default path: `~/.ssh/id_ed25519`
- Make sure the key is added to your AUR account

## Usage

### Build

```bash
cargo build --release
```

### Run

```bash
cargo run
```

Or run the compiled binary:

```bash
./target/release/aur-aacs-keydb
```

## Package Information

- **Package name**: `aacs-keydb-daily`
- **Description**: Contains the Key Database for the AACS Library (Daily Updates)
- **Architecture**: any
- **Dependencies**: libaacs
- **License**: Based on upstream project
- **URL**: <http://fvonline-db.bplaced.net/>

## Workflow

1. **Request archive**: Request creation of new archive of `http://fvonline-db.bplaced.net/export/keydb_eng.zip` on web.archive.org
2. **Get archive URL**: Retrieve the archive URL from web.archive.org
   - Example: `https://web.archive.org/web/20250707095314/http://fvonline-db.bplaced.net/export/keydb_eng.zip`
   - Archive URL is used to download the file
   - Archive timestamp is used to generate the version number (YYYYMMDDhhmmss)
3. **Download and verify**: Download file from archive URL and calculate SHA256
4. **Clone/update AUR repo**: Clone or update the AUR repository
5. **Compare hash**: Compare with current AUR package SHA256 (if package exists)
6. **Generate files**: If different or package doesn't exist, generate PKGBUILD and .SRCINFO
7. **Commit and push**: Automatically commit and push changes to AUR

## Module Structure

- `app.rs`: Core application logic and workflow orchestration, uses `tracing::info` for logging
- `archive.rs`: Web Archive API interaction, handles archive creation and retrieval, uses `tracing::info` for detailed operation logging
- `aur.rs`: AUR package management functionality, handles PKGBUILD and .SRCINFO generation
- `config.rs`: Configuration management, reads environment variables and validates settings
- `git.rs`: Git operations helper, handles repository cloning/updating and commits, uses `tracing::info` for operation status
- `main.rs`: Main application entry point and tracing initialization with hierarchical logging configuration
- `error.rs`: Custom error type definitions and Result type

## Logging

This application uses the `tracing` library for structured, hierarchical logging:

- Logs are formatted with timestamps and module paths
- Log level can be controlled via environment variables (e.g., `RUST_LOG=debug`)
- Default log level is INFO
- All modules use consistent logging (no direct println! calls)
- Logs can be redirected to files or syslog by modifying the tracing subscriber in main.rs

## Notes

- Current version automatically commits and pushes to AUR
- The tool clones/updates the AUR repository automatically
- Generated files are in `/tmp/aur-aacs-keydb-daily/` directory
- Network connection is required (for web.archive.org access)
- SSH key must be configured for AUR access

## License

This project is open source under [0BSD](https://opensource.org/licenses/0BSD).
