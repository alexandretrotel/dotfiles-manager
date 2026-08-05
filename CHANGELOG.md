# Changelog

All notable changes to this project are documented in this file.

## v5.0.0

### Added
- `dotfm` is now usable as a library: operations live in `dotfm::backup`, `dotfm::restore`, `dotfm::sync`, `dotfm::git`, `dotfm::doctor`, `dotfm::profiles`, `dotfm::keyring`, `dotfm::encryption`, and `dotfm::registry`, take a `Dotfm` context (custom root supported via `Dotfm::with_root`), and return report structs instead of printing. The CLI is a thin binary behind the default `cli` feature; depend on the library with `default-features = false`.

### Changed
- The default config registry no longer includes entries for Ghostty, Vim, VS Code (settings and keybindings), Zed, or Git config (and Ghostty's macOS-specific path logic was removed). The default package registry no longer includes entries for Homebrew casks, pnpm, Bun, Deno, or pip. Existing registry files are unaffected.
- **Breaking:** The crate, binary, and repository have been renamed from `mntn` to `dotfm`. Install with `cargo install dotfm` and invoke the CLI as `dotfm`.
- **Breaking:** The data directory moved from `~/.mntn` to `~/.dotfm`. Existing users must move it manually: `mv ~/.mntn ~/.dotfm`.
- **Breaking:** The encrypted bundle filename changed from `mntn-encrypted-bundle.age` to `dotfm-encrypted-bundle.age`. Rename the file inside your backup directory before running `dotfm restore`.
- **Breaking:** The system keychain service name changed from `mntn` to `dotfm`. Re-save your encryption password with `dotfm secret set`.
- **Breaking:** The profile override environment variable was renamed from `MNTN_PROFILE` to `DOTFM_PROFILE`.
- **Breaking:** Restore and `dotfm doctor` no longer fall back to legacy per-file `<source_path>.age` backups when the encrypted bundle is missing or unreadable. Encrypted restore and validation now depend solely on the `dotfm-encrypted-bundle.age` bundle; run `dotfm backup` to produce one if you still have per-file `.age` backups from before v3.1.0.

## v4.0.6

### Changed
- The published crate now uses an explicit `include` allowlist instead of an `exclude` denylist, so only `src/`, `Cargo.toml`, `README.md`, `LICENSE`, and `CHANGELOG.md` are shipped. `.gitignore` and any future non-source files no longer end up in the package.

## v4.0.2

### Changed
- Replaced `anyhow` with `eyre` and `color-eyre` for error handling. Failures now render a colored report with the full cause chain (native `wrap_err` API) and actionable suggestions (e.g. unknown profile, missing git repository).

### Fixed
- Commands now exit with a non-zero status when they fail. Previously errors were caught and printed but the process still exited `0`, so shell chains (`dotfm a && dotfm b`) and CI could not detect failures.

## v4.0.0

### Changed
- **BREAKING:** Renamed the **`validate`** command to **`doctor`**. Run `dotfm doctor` instead of `dotfm validate`; behavior is unchanged. `--ask-password` and `--skip-encrypted` now live on `doctor`.

### Added
- **`dotfm doctor fix`** reformats valid JSON config files with `serde_json`'s pretty printer (atomic write). It normalizes formatting only — it cannot repair true syntax errors, which are reported as unfixable. Pass `--dry-run` to preview changes without writing.

## v3.2.1

### Added
- **`dotfm secret delete`** removes the stored encryption passphrase from the OS keychain (via [keyring](https://crates.io/crates/keyring) `Entry::delete_credential`). If nothing was stored, the command still succeeds.

## v3.2.0

### Added
- **`dotfm secret set`** stores the encryption passphrase in the OS keychain (macOS Keychain, Windows Credential Manager, or Linux Secret Service). Run it once to skip passphrase prompts on later **`backup`**, **`restore`**, and **`validate`** when encrypted configs are involved.
- **`--ask-password`** on **`backup`**, **`restore`**, and **`validate`** forces an interactive passphrase prompt instead of using the stored one.

### Changed
- When no passphrase is stored yet, dotfm still prompts as before and prints a short tip pointing to **`dotfm secret set`**.

## v3.1.0

### Changed
- **Encrypted backups** now produce a single file per encrypted backup root: `dotfm-encrypted-bundle.age`. It contains a tar archive of all registered sensitive files (one passphrase-based age encryption for the whole bundle, so backup and restore spend much less time in key derivation). Paths inside the archive match `source_path` entries in `encrypted.registry.json`.
- **Restore** and **validate** prefer the bundle when present. Older per-path backups (`<source_path>.age`) are still supported if the bundle is missing or cannot be read.
- **`dotfm sync` default commit message** now appends the UTC date and time, for example `chore: sync dotfm (2026-04-04 14:30:00 UTC)`. Pass `--message` to use a fixed subject without a timestamp.

### Added
- **Parallel package backup:** enabled package-manager export commands run concurrently; results are still reported in stable order.

## v3.0.1

### Added
- Added `dotfm sync` to stage all changes under `~/.dotfm`, commit them with `chore: sync dotfm` by default, and push the repository (use `--message` to override).

## v3.0.0

### Breaking
- Removed `install`, `setup`, and `migrate` commands.
- Removed `delete` and `purge` commands.
- Removed `biometric-sudo` command.
- Removed `clean` command.
- Removed `registry` command.
- Replaced `git` subcommands with passthrough `dotfm git <args>`.
- Unified `registry-configs`, `registry-packages`, and `registry-encrypted` into `registry --type`.
- Renamed registry and profile files.
- Removed encrypted filename support for encrypted registry entries. Encrypted files now always use plain relative paths with `.age`.
  - Reason: dotfm is not meant for extra sensitive files. It is fine to back up SSH config (content is encrypted), but if you need filename encryption you likely should not back it up in your dotfiles repo.
  - Migration: encrypted backups that used filename hashing will not be found. Run `dotfm backup` again to recreate them.

### Migration
1. Rename `~/.dotfm/profile.json` to `~/.dotfm/profiles.json`.
2. Rename `~/.dotfm/configs_registry.json` or `~/.dotfm/configs.registry.json` to `~/.dotfm/config.registry.json`.
3. Rename `~/.dotfm/package_registry.json` to `~/.dotfm/package.registry.json`.
4. Rename `~/.dotfm/encrypted_registry.json` or `~/.dotfm/encrypted_configs_registry.json` to `~/.dotfm/encrypted.registry.json`.

### Commands
```bash
mv ~/.dotfm/profile.json ~/.dotfm/profiles.json

safe_migrate_registry() {
  local destination="$1"
  shift

  local existing_sources=()
  local source
  for source in "$@"; do
    if [ -e "$source" ]; then
      existing_sources+=("$source")
    fi
  done

  if [ "${#existing_sources[@]}" -gt 1 ]; then
    echo "WARN: Multiple source files found for $destination: ${existing_sources[*]}. Skipping to avoid overwrite."
    return 1
  fi

  if [ "${#existing_sources[@]}" -eq 1 ]; then
    if [ -e "$destination" ]; then
      echo "WARN: Destination already exists: $destination. Skipping ${existing_sources[0]}."
      return 1
    fi

    mv "${existing_sources[0]}" "$destination"
  fi
}

safe_migrate_registry ~/.dotfm/config.registry.json \
  ~/.dotfm/configs_registry.json \
  ~/.dotfm/configs.registry.json

safe_migrate_registry ~/.dotfm/package.registry.json \
  ~/.dotfm/package_registry.json

safe_migrate_registry ~/.dotfm/encrypted.registry.json \
  ~/.dotfm/encrypted_registry.json \
  ~/.dotfm/encrypted_configs_registry.json
```

### Changed
- Switched project license from MIT to GNU GPL v3.0 or later (Free Software Foundation).

### Added
- Initialize git repository when `dotfm backup` creates `~/.dotfm`.

## v2.3.0

### Added
- **Sync Diff Views:** Added `dotfm sync --diff` to show combined unstaged and staged changes.
  - Uses `--cached` fallback for older git versions when showing staged diffs.

### Fixed
- **Package Registry Output:** Strips ANSI escape codes from package registry command output to keep package registry output clean.

## v2.2.0

### Added
- **File Mismatch Validation:** The `validate` command now automatically compares current filesystem files with their backups in `~/.dotfm/backup/` and warns if they differ. Supports both regular and encrypted registry entries, with password prompting for encrypted files. Helps detect unsaved changes and configuration drift.

## v2.1.0

### Added
- **Encrypted Configuration Registry:** Added secure password-based encryption for sensitive configuration files (SSH keys, credentials) with age encryption, filename encryption support, and seamless integration with backup/restore commands.

## v2.0.0

### Added
- **Interactive Setup Wizard:** Introduced a user-friendly wizard for configuring dotfiles management.
- **Profile Management:** Added support for multiple profiles, including default profile saving and migration tasks for layered backup structures.
- **Validation Command:** New `validate` command to check configuration integrity.
- **Dry-Run Functionality:** Added dry-run flags and execution for biometric sudo, configs registry, package registry, delete, and validate tasks.
- **Parallel Backup:** Enabled parallel operations for backup tasks.
- **Sync Command:** Added a `sync` command with Git repository management options, including initialization and auto-commit features.
- **VS Code Extensions:** Backup and restore functionality for VS Code extensions.
- **Zed Settings:** Added registry entry and path helper for Zed editor settings.
- **Git Configuration:** Added Git configuration entry to ConfigsRegistry.
- **Cross-Platform Trash Cleaning:** Implemented trash cleaning for macOS, Linux, and enhanced Windows support.
- **Startup Program Listing:** Implemented listing of startup programs on Windows.
- **Comprehensive Logging:** Enhanced logging with error, success, and warning messages.
- **CI Workflow:** Added GitHub Actions CI workflow.

### Changed
- **Refactored CLI Arguments:** Improved argument structures for backup, migration, and link tasks.
- **Registry Management:** Refactored to use `ConfigsRegistry` and improved registry structure.
- **Category Handling:** Optimized category filtering, parsing, and display logic.
- **Code Structure:** Major refactor of core modules, including moving and renaming files for better organization (e.g., registry modules, tasks).
- **Documentation:** Expanded and improved README with detailed guides and usage examples.
- **Error Handling:** Improved error handling and logging throughout the codebase.
- **Platform Compatibility:** Enhanced platform-specific code for better cross-OS support.
- **Project Formatting:** Applied consistent formatting and code cleanup.

### Removed
- **Redundant Registry Logic:** Removed redundant abstractions and unused methods.
- **Git Integration in Enhancements Doc:** Removed outdated documentation about git integration and sync command.

### Fixed
- **Windows Support:** Fixed and improved Windows-specific logic and tests.
- **CI and Linting:** Fixed CI workflow permissions and linting errors.
- **Code Cleanups:** Removed unused variables, simplified control flows, and improved consistency in method signatures.
- **Platform-Specific Bugs:** Addressed various platform-specific bugs and improved compatibility.
