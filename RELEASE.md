# Release guide

Compass follows semantic versioning. The `Create Release` workflow updates `Cargo.toml`, `Cargo.lock`, AppStream releases, and `cargo-sources.json`, then creates a `vX.Y.Z` tag. The tag starts the `Release` workflow, which builds native/cross binaries, a Flatpak bundle, an APKBUILD, and a source archive.

## Automated release

1. Run **Create Release** from GitHub Actions.
2. Enter a version or leave it empty to increment the patch component.
3. Confirm the metadata update and tag complete.
4. Confirm all release jobs pass before publishing downstream packaging updates.

The workflow needs `RELEASE_TOKEN` to push a tag that starts another workflow. Optional Flathub publishing needs a token with access to `flathub/io.github.cosmic_utils.compass`.

## Manual fallback

Update the version and release entry, run `cargo update --workspace`, `just generate`, and `just check`, then commit and tag outside this working tree only after review.

Release assets use the `compass-<architecture>-linux.tar.gz`, `compass-<architecture>.flatpak`, and `compass-vX.Y.Z-source.zip` naming scheme.
