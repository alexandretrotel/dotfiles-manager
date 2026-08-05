# dotfm

dotfm is built to keep your dotfiles organized, safe, and consistent across machines using profiles.

A profile is a named set of configuration choices that represents a context, like work, personal, or minimal. With profiles, you can keep multiple setups and switch between them so the right settings are active for the situation.

At a high level, dotfm helps you manage these configurations, keep them in sync, and recover them when needed.

![Demo Video](./assets/dotfm.gif)

## Quick Start

```bash
dotfm backup
dotfm restore
dotfm doctor
```

Switch profiles:

```bash
dotfm profile create work --description "Work setup"
dotfm use work
```

## Core Commands

- `backup` - copy tracked configs into `~/.dotfm/backup/`
- `restore` - restore configs from backup
- `doctor` - check registry files and config drift; `doctor fix` reformats valid JSON configs with serde_json's pretty printer (it cannot repair true syntax errors, only normalize formatting)
- `secret` - store (`secret set`) or remove (`secret delete`) the encryption passphrase in the OS keychain so `backup` / `restore` / `doctor` can reuse it without prompting
- `profile` - list/create/delete profiles
- `use` - switch active profile
- `git` - run any git command inside `~/.dotfm`
- `sync` - run `git add .`, commit with default message `chore: sync dotfm (YYYY-MM-DD HH:MM:SS UTC)` (use `--message` to override), then `git push` inside `~/.dotfm`

Encrypted configs: run `dotfm secret set` after you know your passphrase to persist it. Use `--ask-password` on `backup`, `restore`, or `doctor` if you want to type it for that run only.

## Directory Layout

```text
~/.dotfm/
├── backup/
│   ├── common/
│   │   └── encrypted/          # optional: encrypted bundle + legacy per-file .age
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
