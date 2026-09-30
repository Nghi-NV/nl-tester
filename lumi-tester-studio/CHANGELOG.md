# Lumi IDE Changelog

## [0.2.0] - 2026-09-30

### Added
- Added an About & Updates page with the installed version, Lumi IDE release notes, and a check for newer Lumi IDE releases.
- Added direct access to the release download page when a newer version is available.

### Fixed
- Closing the last window now exits the app process; background process cleanup runs as the app exits.
- If automatic save fails while closing, the app now offers a clear choice to close and discard the unsaved edits.

### Update behavior
- This release checks GitHub for updates and opens the release page to download an installer. Signed in-app installation is not enabled yet.

## [0.1.4] - 2026-09-30

### Fixed
- Removed the unsupported `--relative` option from Git status so the Source Control view works with the installed Git CLI.
- Improved Codex CLI response handling when the command returns an empty standard output stream.
- Improved shutdown handling for active AI requests, test runs, and integrated terminals.
- Updated the product and executable name to Lumi IDE.
