bureaucrat
==========

Automatically add issue tracking IDs to your Git commits.

Inspired by [pre-commit](https://pre-commit.com/) and its ability to make life easier.

## Installation

Precompiled binaries for Linux and macOS are available on [GitHub releases](https://github.com/arttuperala/bureaucrat/releases).

On macOS, you can install bureaucrat using [Homebrew](https://brew.sh/):

```
brew install arttuperala/bureaucrat/bureaucrat
```

## Usage

To start using bureaucrat, you need to (1) configure bureaucrat in your Git repository and (2) install the prepare-commit-msg hook. To install the hook, run `bureaucrat install` inside your Git repository.

After both steps have been completed, all you need to do is add your issue tracking code to your branch name and it will automatically be included in your Git commit message. You can use a prefix in your branch name and omit the separator character from your branch name, so for "GH" issue tracking prefix, you can name your branch `GH-1234-feature`, `GH1234-feature`, `feature/GH1234-feature`, or `feature/GH-1234-feature`, and it will be handled.

In addition to the configured issue tracking codes, bureaucrat automatically handles CVE identifiers (e.g. `feature/CVE-2025-12345`).

## Configuration

bureaucrat is configured by adding a YAML configuration file in your Git repository root where you specify the issue tracking prefixes that you use under `codes`. You can also optionally set bureaucrat to only work on certain branch prefixes under `branch_prefixes`, so `feature/GH-123` branch would be tagged but `GH-123` branch wouldn't if `branch_prefixes` was set to `["feature"]`.

Possible filenames for the configuration:

- .bureaucrat-config.yaml
- .bureaucrat-config.yml
- .bureaucrat.yaml
- .bureaucrat.yml

### Example configuration

```yaml
codes:
  - GH
branch_prefixes:
  - bugfix
  - feature
```

## License

bureaucrat is available under the MIT License. For full license text, see the [`LICENSE` file](LICENSE).

---

<p align="center"><i>Branches are fleeting, but commits are forever.</i></p>
