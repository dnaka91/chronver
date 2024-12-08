<!-- markdownlint-disable MD024 -->

# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - 2024-12-08

- Mark project as deprecated.

## [0.2.0] - 2021-10-23

### Changed

- Switch to Rust edition 2021 (MSRV is `1.56` now).

### Security

- Replace `chrono` with `time` due to recent security issues and, as a side effect, better parsing
  performance.

## [0.1.0]

### Added

- Initial release.

[Unreleased]: https://github.com/dnaka91/chronver/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/dnaka91/chronver/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/dnaka91/chronver/releases/tag/v0.1.0
