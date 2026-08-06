# dotfiles-manager

dotfiles-manager (aliased as `dfm`) is built to keep your dotfiles organized, safe, and consistent across machines using profiles.

A profile is a named set of configuration choices that represents a context, like work, personal, or minimal. With profiles, you can keep multiple setups and switch between them so the right settings are active for the situation.

At a high level, dotfiles-manager helps you manage these configurations, keep them in sync, and recover them when needed.

![Demo Video](./assets/dfm.gif)

## Quick Start

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
| `backup`  | Copy tracked configs into `~/.dfm/backup/`                                |
| `restore` | Restore configs from backup                                               |
| `doctor`  | Check registry files and config drift                                     |
| `profile` | List, create, or delete profiles                                          |
| `use`     | Switch the active profile                                                 |
| `git`     | Run any git command inside `~/.dfm`                                       |
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
