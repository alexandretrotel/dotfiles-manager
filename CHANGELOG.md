# Changelog

All notable changes to this project are documented in this file.

## v4.0.2

### Changed
- Replaced `anyhow` with `eyre` for error handling (native `wrap_err` API).

## v4.0.0

### Changed
- **BREAKING:** Renamed the **`validate`** command to **`doctor`**. Run `mntn doctor` instead of `mntn validate`; behavior is unchanged. `--ask-password` and `--skip-encrypted` now live on `doctor`.

### Added
- **`mntn doctor fix`** reformats valid JSON config files with `serde_json`'s pretty printer (atomic write). It normalizes formatting only — it cannot repair true syntax errors, which are reported as unfixable. Pass `--dry-run` to preview changes without writing.

## v3.2.1

### Added
- **`mntn secret delete`** removes the stored encryption passphrase from the OS keychain (via [keyring](https://crates.io/crates/keyring) `Entry::delete_credential`). If nothing was stored, the command still succeeds.

## v3.2.0

### Added
- **`mntn secret set`** stores the encryption passphrase in the OS keychain (macOS Keychain, Windows Credential Manager, or Linux Secret Service). Run it once to skip passphrase prompts on later **`backup`**, **`restore`**, and **`validate`** when encrypted configs are involved.
- **`--ask-password`** on **`backup`**, **`restore`**, and **`validate`** forces an interactive passphrase prompt instead of using the stored one.

### Changed
- When no passphrase is stored yet, mntn still prompts as before and prints a short tip pointing to **`mntn secret set`**.

## v3.1.0

### Changed
- **Encrypted backups** now produce a single file per encrypted backup root: `mntn-encrypted-bundle.age`. It contains a tar archive of all registered sensitive files (one passphrase-based age encryption for the whole bundle, so backup and restore spend much less time in key derivation). Paths inside the archive match `source_path` entries in `encrypted.registry.json`.
- **Restore** and **validate** prefer the bundle when present. Older per-path backups (`<source_path>.age`) are still supported if the bundle is missing or cannot be read.
- **`mntn sync` default commit message** now appends the UTC date and time, for example `chore: sync mntn (2026-04-04 14:30:00 UTC)`. Pass `--message` to use a fixed subject without a timestamp.

### Added
- **Parallel package backup:** enabled package-manager export commands run concurrently; results are still reported in stable order.

## v3.0.1

### Added
- Added `mntn sync` to stage all changes under `~/.mntn`, commit them with `chore: sync mntn` by default, and push the repository (use `--message` to override).

## v3.0.0

### Breaking
- Removed `install`, `setup`, and `migrate` commands.
- Removed `delete` and `purge` commands.
- Removed `biometric-sudo` command.
- Removed `clean` command.
- Removed `registry` command.
- Replaced `git` subcommands with passthrough `mntn git <args>`.
- Unified `registry-configs`, `registry-packages`, and `registry-encrypted` into `registry --type`.
- Renamed registry and profile files.
- Removed encrypted filename support for encrypted registry entries. Encrypted files now always use plain relative paths with `.age`.
  - Reason: mntn is not meant for extra sensitive files. It is fine to back up SSH config (content is encrypted), but if you need filename encryption you likely should not back it up in your dotfiles repo.
  - Migration: encrypted backups that used filename hashing will not be found. Run `mntn backup` again to recreate them.

### Migration
1. Rename `~/.mntn/profile.json` to `~/.mntn/profiles.json`.
2. Rename `~/.mntn/configs_registry.json` or `~/.mntn/configs.registry.json` to `~/.mntn/config.registry.json`.
3. Rename `~/.mntn/package_registry.json` to `~/.mntn/package.registry.json`.
4. Rename `~/.mntn/encrypted_registry.json` or `~/.mntn/encrypted_configs_registry.json` to `~/.mntn/encrypted.registry.json`.

### Commands
```bash
mv ~/.mntn/profile.json ~/.mntn/profiles.json

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

safe_migrate_registry ~/.mntn/config.registry.json \
  ~/.mntn/configs_registry.json \
  ~/.mntn/configs.registry.json

safe_migrate_registry ~/.mntn/package.registry.json \
  ~/.mntn/package_registry.json

safe_migrate_registry ~/.mntn/encrypted.registry.json \
  ~/.mntn/encrypted_registry.json \
  ~/.mntn/encrypted_configs_registry.json
```

### Changed
- Switched project license from MIT to GNU GPL v3.0 or later (Free Software Foundation).

### Added
- Initialize git repository when `mntn backup` creates `~/.mntn`.

## v2.3.0

### Added
- **Sync Diff Views:** Added `mntn sync --diff` to show combined unstaged and staged changes.
  - Uses `--cached` fallback for older git versions when showing staged diffs.

### Fixed
- **Package Registry Output:** Strips ANSI escape codes from package registry command output to keep package registry output clean.

## v2.2.0

### Added
- **File Mismatch Validation:** The `validate` command now automatically compares current filesystem files with their backups in `~/.mntn/backup/` and warns if they differ. Supports both regular and encrypted registry entries, with password prompting for encrypted files. Helps detect unsaved changes and configuration drift.

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
