# ✨ Singularity Dotfiles Manager

An extendable CLI tool designed to automate the installation and management of your system configuration and application ecosystem. Built with Rust, and powered by the **Rhai** scripting engine for ultimate flexibility.

---

## 🚀 Key Features

- **Release-Based Configuration**: Deploy entire dotfile environments from a single JSON release config.
- **Programmable Hooks**: Use the built-in **Rhai Hook API** to run logic at specific points during the installation process.
- **App Suites**: Install from curated lists of applications tailored to your specific dotfiles.
- **Developer Friendly**: Full API documentation.
- **Cross-Distribution Ready**: Intelligent path resolution (including `~`) and distribution detection.

---

## 🛠 Installation

```bash
cargo install --git https://github.com/JTreske/singularity-dotfiles-manager.git
```

---

## 📖 Usage Guide

Find help using the `singularity-dotfiles-manager` by running:

```bash
singularity-dotfiles-manager --help
```

### Install Dotfiles

The core command of Singularity. It pulls a dotfiles repository based on a configuration file.

```bash
singularity-dotfiles-manager install https://example.com/release-config.json
```

### App Suite

Let the user install from a suite of applications defined in your profile.

```bash
singularity-dotfiles-manager apps
```

---

## ⚓ Hook Scripting API

Singularity provides a powerful **Hook API** accessible via Rhai scripts (e.g., `hooks.rhai`). This allows you to interact with the system during the installation process.

### Available API Modules

| Module         | Purpose                                                     |
| :------------- | :---------------------------------------------------------- |
| `api::log`     | Beautiful terminal output (steps, warnings, successes).     |
| `api::runner`  | Install system packages, clone git repos, and run commands. |
| `api::utility` | File I/O, JSON parsing, regex, and interactive prompts.     |

---

## ⚙️ Configuration

### Release Config JSON

The manager expects a JSON structure defining your dotfiles release.
A minimal release configuration may look like this:

```json
{
  "name": "Example Dotfiles (Rolling)",
  "id": "org.example.dotfiles.rolling",
  "repository": "https://github.com/example/dotfiles.git",
  "dotfiles": "dotfiles"
}
```

---

## 📚 Documentation

A comprehensive guide how to use the Singularity Dotfiles Manager is available in the [Docs]().

### Viewing Locally

If you are developing locally, you can generate and open the documentation with:

```bash
# Generate the API metadata and MDX files
cargo run --bin gen_docs --features="rhai-autodocs"

# Install mdBook
cargo install mdbook

# Serve the book
mdbook serve docs --open
```

---

## 📜 License

Distributed under the GPLv3 License. See [LICENSE](LICENSE.md) for more information.
