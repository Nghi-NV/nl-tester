# e2e

This directory contains glue code for testing Maestro itself.

## Testing

Typical workflow is:

1. Start Android emulator and iOS simulator
2. `download_apps`
3. `install_apps`
4. `run_tests`

We try to keep shell code in separate files, so we don't get too tightly coupled
to GitHub Actions.

### Expected failures

Let's say a critical bug is introduced that causes Maestro to always mark all
tests as passed. If our e2e test suite only was only checking if all tests pass
(i.e. `maestro test` exit code is 0), then wouldn't catch such a bug.

To prevent this, all flows in this directory MUST have a `passing` or `failing`
label, so the correct outcome can be asserted.

## Samples

This directory also contains samples that are downloaded by the `maestro download-samples` command,
and some glue code to facilitate updating those samples.

`maestro download-samples` provides a set of flows and apps so that users can
quickly try out Maestro, without having to write any flows for their own app.

`download-samples` downloads these files and apps from our publicly-available
Google Cloud Storage bucket (hosted on `storage.googleapis.com`).

### Intro

The samples are automatically updated by the GitHub Action on every new commit
to the `main` branch.

There zip archive that is downloaded by `download-samples` consists of 2 things:
- the Maestro workspace with flows (located in the `workspaces/wikipedia` directory)
- the app binary files that are used in the flows (located in the `apps` directory)

App binary files are heavy, so we don't store them in the repository. Instead, they are hosted
on publicly available directory in Google Cloud Storage:

### Update the samples

Run the script:

```console
./update_samples
```

## VS Code Extension Installation

The `lumi-tester-vscode` extension provides CodeLens buttons for running tests directly from YAML files.

### Build and Package

```bash
cd lumi-tester-vscode
npm run compile
npx vsce package --allow-missing-repository
```

### Install on VS Code

```bash
code --install-extension lumi-tester-vscode/lumi-tester-*.vsix --force
```

### Install on Antigravity

```bash
/Applications/Antigravity.app/Contents/Resources/app/bin/antigravity --install-extension lumi-tester-vscode/lumi-tester-*.vsix --force
```

After installation, reload the editor (`Cmd+Shift+P` → "Developer: Reload Window").
