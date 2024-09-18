# Changelog

All notable changes to this project will be documented in this file.

## [0.8.0] - 2024-09-18

### 🚀 Features

- [**breaking**] Refactor BlockingClient to Client add ClientBuilder
- Improve Client error handling logic
- _(filter_builder)_ Add `.filter_builder()` and `.filter()` convenience methods

### 🚜 Refactor

- Client module
- [**breaking**] Update the library interfaces
- Module structure and update docs

### 📚 Documentation

- Add lib_demo.rs example

### 🧪 Testing

- Add environment variable configuration test and combine getter and setter method tests
- Add integration tests

### ⚙️ Miscellaneous Tasks

- Run tests on all branches except main
- Log test errors

## [0.7.0] - 2024-08-07

### 🚀 Features

- _(platforms)_ Drop OpenSSL requirement and add support for Musl Linux and Linux on arm64 (AArch64)
  - New supported platforms:
    - `aarch64-unknown-linux-gnu`
    - `aarch64-unknown-linux-musl`
    - `x86_64-unknown-linux-musl`

### ⚙️ Miscellaneous Tasks

- _(docs)_ Update README

## [0.6.1] - 2024-07-30

### ⚙️ Miscellaneous Tasks

- _(release)_ Configure release-plz to update dependencies
- _(docs)_ Update README

## [0.6.0] - 2024-07-28

### 🚀 Features

- Retry failed requests to get AWS IP Ranges from URL

## [0.5.4] - 2024-07-27

### ⚙️ Miscellaneous Tasks

- _(release)_ Automate the release process
- _(release)_ Update method for merging pull requests in mergify.yml
- _(release)_ Mergify require all checks to pass before auto merge
- _(release)_ Quote Mergify conditions with modifiers
- _(mergify)_ Check failures must be equal to zero
- _(release)_ Update token used for checkout action
