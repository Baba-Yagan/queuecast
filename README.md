# queuecast

A command-line tool for managing TV show files with weekly scheduling.

## Building

Build the project in release mode:
```bash
make build
```

For development builds:
```bash
make dev
```

## Installation

### User Installation
Install to your local Cargo bin directory (`~/.cargo/bin`):
```bash
make install
```

### System-wide Installation
Install system-wide to `/usr/local/bin` (requires sudo):
```bash
make install-system
```

## Uninstalling

Remove from user directory:
```bash
make uninstall
```

Remove from system directory:
```bash
make uninstall-system
```

## Development

Run the program during development:
```bash
make run
```

Clean build artifacts:
```bash
make clean
```

## Usage

queuecast helps you manage TV show directories and automatically creates symlinks for weekly episode scheduling.

Basic commands:
- `queuecast add <directory>` - Add a TV show directory
- `queuecast list` - List all programs
- `queuecast config symlink-dir <path>` - Set symlink directory
- `queuecast update` - Update symlinks for scheduled episodes

For more information, run `queuecast --help`.
