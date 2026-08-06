# Changelog

All notable changes to this project are documented in this file.

## v1.1.0

### Added
- **`dfm prune`** deletes backup directories left behind by profiles that no longer exist (e.g. a profile removed with `dfm profile delete`, which intentionally keeps its backup directory on disk). It lists the orphaned directories it finds and asks for confirmation — defaulting to "no" — before deleting anything.
- **`dfm doctor --include-disabled`** also runs the backup consistency check against disabled registry entries, which are skipped by default.
- **`dfm edit <registry>`** opens one of dfm's own registry/config files (`config`, `package`, `encrypted`, or `profiles`) in an editor. The editor is chosen from `--editor`/`-e`, then `$VISUAL`, then `$EDITOR`, falling back to `vi`; it can be a known name like `nano` or `emacs`, or any custom binary/command (e.g. `code --wait`).

### Changed
- `dfm backup` no longer rewrites the encrypted bundle (`dfm-encrypted-bundle.age`) when its contents haven't actually changed. `age` encryption uses a fresh salt/nonce every run, so previously the ciphertext changed on every backup regardless of whether the underlying dotfiles did, and `dfm sync` committed a near-full copy of the bundle every time — bloating the dfm repo on a long run of backup-only syncs. A small plaintext hash file (`dfm-encrypted-bundle.sha256`) is now kept alongside the bundle to detect when the archived content is unchanged; when it is, the bundle is left untouched and `dfm sync` has nothing to commit for it. Real content changes still produce a normal commit, exactly as before.

## v1.0.0

Initial release of `dotfiles-manager` (aliased as `dfm`).

### Added
- Profile-based dotfiles management: named profiles (e.g. work, personal, minimal) represent a context; `dfm profile create`/`delete` manage them and `dfm use` switches the active one.
- `dfm backup` copies tracked configs into `~/.dfm/backup/`; `dfm restore` restores them.
- `dfm link <repo>` clones a dotfiles repo (URL or `owner/repo`) into `~/.dfm` and restores it in one step, for new-machine setup.
- `dfm doctor` validates `dotfiles-manager`'s own registry files (`config.registry.json`, `package.registry.json`) and checks backups against the current filesystem, comparing directory config entries file-by-file and recursively. `dfm doctor --fix` rewrites the registry and profile files as pretty-printed, deterministically-sorted JSON. `--ask-password` and `--skip-encrypted` control how encrypted entries are checked.
- Encrypted configuration registry: sensitive files (SSH keys, credentials, etc.) are stored as a single passphrase-encrypted bundle (`dfm-encrypted-bundle.age`) via `age`; entries may target whole directories, not just single files. `dfm secret set`/`secret delete` store or remove the passphrase in the OS keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service) so `backup`/`restore`/`doctor` don't need to prompt each run.
- `dfm git <args>` runs any git command inside `~/.dfm`; `dfm status` and `dfm diff` are shortcuts for `dfm git status`/`dfm git diff`, with extra args passed through.
- `dfm sync` stages, commits, and pushes changes inside `~/.dfm`, with a default UTC-timestamped commit message.
- Usable as a library: operations live in `dotfiles_manager::backup`, `dotfiles_manager::restore`, `dotfiles_manager::sync`, `dotfiles_manager::git`, `dotfiles_manager::doctor`, `dotfiles_manager::profiles`, `dotfiles_manager::keyring`, `dotfiles_manager::encryption`, and `dotfiles_manager::registry`, taking a `Dfm` context (custom root via `Dfm::with_root`) and returning report structs instead of printing. The CLI ships two binaries, `dotfiles-manager` and the shorter alias `dfm`, behind the default `cli` feature; depend on the library with `default-features = false`.
- Licensed under GPL-3.0-or-later.
