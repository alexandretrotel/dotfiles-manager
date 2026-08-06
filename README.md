# dotfiles-manager

dotfiles-manager (aliased as `dfm`) is built to keep your dotfiles organized, safe, and consistent across machines using profiles.

A profile is a named set of configuration choices that represents a context, like work, personal, or minimal. With profiles, you can keep multiple setups and switch between them so the right settings are active for the situation.

At a high level, dotfiles-manager helps you manage these configurations, keep them in sync, and recover them when needed.

![Demo Video](https://raw.githubusercontent.com/alexandretrotel/dotfiles-manager/main/assets/dfm.gif)

## Installation

```bash
cargo install dotfiles-manager
```

This builds and installs both binaries: `dotfiles-manager` and the shorter alias `dfm`.

Prebuilt binaries for Linux, macOS (Intel and Apple Silicon), and Windows are also available on the [Releases page](https://github.com/alexandretrotel/dotfiles-manager/releases).

## Quick Start

Setting up a new machine? Clone your existing dotfiles repo and restore in one step:

```bash
dfm link owner/repo
```

Otherwise:

```bash
dfm backup
dfm restore
dfm doctor
```

Switch profiles:

```bash
dfm profile create work --description "Work setup"
dfm use work
```

## Core Commands

| Command   | What it does                                                             |
| --------- | ------------------------------------------------------------------------- |
| `link`    | Clone a dotfiles repo into `~/.dfm` and restore it (new machine setup)    |
| `backup`  | Copy tracked configs into `~/.dfm/backup/`                                |
| `restore` | Restore configs from backup                                               |
| `doctor`  | Check registry files and config drift                                     |
| `profile` | List, create, or delete profiles                                          |
| `use`     | Switch the active profile                                                 |
| `git`     | Run any git command inside `~/.dfm`                                       |
| `status`  | Shortcut for `dfm git status`                                             |
| `diff`    | Shortcut for `dfm git diff`                                               |
| `sync`    | Commit and push changes inside `~/.dfm`                                   |
| `secret`  | Store or remove the encryption passphrase in the OS keychain              |

**Encrypted configs:** run `dfm secret set` once you know your passphrase to persist it. Pass `--ask-password` to `backup`, `restore`, or `doctor` to type the passphrase for that run instead (bypassing the keychain) — encrypted files are still processed either way.

## Directory Layout

```text
~/.dfm/
├── backup/
│   ├── common/
│   │   └── encrypted/          # optional: encrypted bundle
│   └── profiles/
│       └── <name>/
│           └── encrypted/
├── profiles.json
├── .active-profile
├── config.registry.json
├── package.registry.json
└── encrypted.registry.json
```

Registry notes:
- `config.registry.json` tracks regular dotfiles and their targets.
- `package.registry.json` tracks package managers and how to export package lists.
- `encrypted.registry.json` tracks sensitive files that are stored encrypted.

## License

GNU General Public License v3.0 or later (GPL-3.0-or-later), published by the Free Software Foundation.
