# Release process

This project should not keep manually compiled binaries in git. Git stores the source code; GitHub Actions builds release artifacts from a tagged commit and publishes checksums with provenance information.

## Normal release flow

1. Ensure `main` is green in GitHub Actions.
2. Update `Cargo.toml` if the version changes.
3. Create and push a signed or annotated tag:

```bash
git tag -a v0.1.0 -m "Release v0.1.0"
git push origin v0.1.0
```

4. The `Release` workflow builds `target/release/linux-it-guy-toolbox` from that exact tag.
5. The workflow uploads:
   - `linux-it-guy-toolbox-<version>-linux-x86_64.tar.gz`
   - `linux-it-guy-toolbox-<version>-linux-x86_64.tar.gz.sha256`
   - `BUILD-PROVENANCE.txt`

## User verification

Download the `.tar.gz` and matching `.sha256` file from the GitHub release, then run:

```bash
sha256sum -c linux-it-guy-toolbox-<version>-linux-x86_64.tar.gz.sha256
```

The checksum proves the downloaded archive is identical to the asset produced by GitHub Actions. For stronger assurance, compare the release tag, commit SHA, and workflow run listed in `BUILD-PROVENANCE.txt` with the public GitHub Actions run.

## Local build

```bash
cargo build --locked --release
```

The binary is written to:

```text
target/release/linux-it-guy-toolbox
```
