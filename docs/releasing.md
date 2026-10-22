# Publishing to crates.io

The Cargo package and installed executable are both named `aai-cli`.

## Automatic releases

Merges and pushes to `main` run `.github/workflows/publish.yml`. The workflow:

1. Selects the next unused patch version (the first release is `0.1.1`).
2. Dates the current `Unreleased` changelog entries and refreshes `Cargo.lock`.
3. Runs all safe checks and a publication dry run.
4. Commits the version and changelog to `main` using `GITHUB_TOKEN`.
5. Publishes with the repository Actions secret `CARGO_REGISTRY_TOKEN`.
6. Pushes an annotated `v<VERSION>` tag after successful publication.

The workflow is serialized and never cancels an upload in progress. It checks out
latest `main` when it starts, so closely spaced merges can share a release. If
`main` advances during validation, the release push fails before uploading;
rerun the workflow to release the latest code. GitHub's built-in token does not
trigger another push workflow for the version commit.

The repository must allow Actions to push release commits to `main` and version
tags. If branch protection prevents this, adjust the release permissions before
running it. Do not disable required checks for ordinary pull requests.

To recover from an interrupted publication, rerun **Publish crate** in Actions.
When HEAD is already a release commit, it reuses that version. If crates.io
already has it, it skips uploading and completes the tag push. Registry errors
other than a missing crate stop the workflow. No token is stored in the repository.

For a minor or major release, set the manifest to the desired release line before
merging; automatic releases always increment its patch component. Update
`Unreleased` notes with each user-facing change.

## Manual release fallback

1. Choose an unpublished version in `Cargo.toml` and refresh `Cargo.lock` with
   `cargo check --all-targets --all-features`.
2. Move the release's `CHANGELOG.md` entries from `Unreleased` into a dated version
   section.
3. Run the safe checks and inspect the package contents:

   ```bash
   scripts/run-tests.sh safe
   cargo package --list
   cargo publish --dry-run --locked --registry crates-io
   ```

   When validating uncommitted release edits, add `--allow-dirty` to the Cargo
   package and publish dry-run commands. Commit the reviewed release changes
   before the actual upload.

The manifest explicitly includes source, build inputs, bundled skills, tests,
the lockfile, README, changelog, and license. Local configurations and downloaded
provider documentation are excluded. Bundled skills are required because
`build.rs` embeds them in the binary. The dry run builds the extracted package to
verify that it is self-contained.

## Authenticate and publish

Sign in to [crates.io](https://crates.io/), verify the account's email address,
and create an API token with permission to publish `aai-cli`. For the first
release, the token must permit creating the crate. Enter it interactively:

```bash
cargo login --registry crates-io
```

Never put the token in source files, command arguments, or chat. Once the release
commit is ready, publish it:

```bash
cargo publish --locked --registry crates-io
```

Check the version on [crates.io](https://crates.io/crates/aai-cli), tag the release
commit as `v<VERSION>`, and verify installation:

```bash
cargo install aai-cli --version <VERSION> --locked
aai-cli --version
aai-cli skills list
```

A published version cannot be overwritten. See the
[Cargo publishing guide](https://doc.rust-lang.org/cargo/reference/publishing.html)
for registry requirements and release management.
