# Installor v0.1.1

Installor v0.1.1 is a maintenance release that keeps the v0.1.0 product behavior intact while making the repository's native CI checks portable across Windows, macOS, and Linux.

## What changed

- Platform-specific detector implementations are now compiled only on their target operating systems.
- Cross-platform parsing tests remain available without unused native detector stubs.
- Strict Clippy validation succeeds on Windows and is configured for native macOS and Linux runners.
- The project website is deployed through the dedicated GitHub Pages workflow.

## Windows downloads

- `Installor_0.1.1_x64-setup.exe` — guided NSIS installer.
- `Installor_0.1.1_x64_en-US.msi` — Windows Installer package.
- `installor.exe` — standalone executable for portable evaluation.
- `SHA256SUMS.txt` — SHA-256 values for all Windows artifacts.

These community artifacts are not code-signed. Verify the checksum, obtain the files only from the official GitHub release, and review operating-system prompts before continuing.

## Validation scope

The Windows application is built, launched, and tested locally. GitHub Actions exercises formatting, strict linting, and tests on native Windows, macOS, and Ubuntu runners. Signing, notarization, and real-hardware installation QA for macOS and Linux remain future validation work.
