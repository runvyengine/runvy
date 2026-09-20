<!--
?? DEPRECATED � ECS Migration in Progress

This documentation refers to the old OCS (runvy_core::ocs) system.
The engine is migrating to a new archetype-based ECS (runvy_ecs crate).

See ROADMAP.md for the current migration track.
-->

# Development Guide

## Versioning & Releases

### Current Scheme

The project uses [Semantic Versioning 2.0](https://semver.org/): `0.6.0-alpha.1`.

```
MAJOR.MINOR.PATCH-PRERELEASE.COUNT
```

- **MAJOR** — breaking API changes (0 while in initial development)
- **MINOR** — new features, backwards compatible
- **PATCH** — bug fixes, backwards compatible
- **PRERELEASE** — `alpha`, `beta`, `rc`
- **COUNT** — build number within the pre-release stage

### Version Source of Truth

Version is defined once in root `Cargo.toml` under `[workspace.package]`:

```toml
[workspace.package]
version = "0.6.0-alpha.1"
```

Each crate inherits it via `version.workspace = true`.

### Release Process

```
1. Update CHANGELOG.md
   - Move [Unreleased] → new version section with today's date
   - Start a fresh [Unreleased] section above

2. Bump version
   - Edit Cargo.toml workspace version
   - Run: cargo check (catches stale version references)

3. Tag and push
    git commit -am "Release v0.6.0-alpha.2"
    git tag v0.6.0-alpha.2
    git push && git push --tags

4. Create GitHub Release
   - Title: v0.6.0-alpha.2
   - Paste changelog section as release notes
   - Attach the CI-built binaries (see Build section below)
```

### When to Bump

| Change Type           | Bump                       |
| --------------------- | -------------------------- |
| Breaking API change   | MAJOR (or MINOR while 0.x) |
| New feature           | MINOR                      |
| Bug fix               | PATCH                      |
| Pre-release iteration | COUNT                      |

---

## Build System

### Current CI (`.github/workflows/ci.yml`)

Runs on every push/PR to `main`:

- `cargo check --workspace`
- `cargo test --workspace`
- `cargo clippy --workspace -- -D warnings`
- `cargo fmt --all --check`

### Recommended CI Improvements

**Add a release workflow** (`.github/workflows/release.yml`):

```yaml
name: Release
on:
  push:
    tags: ["v*"]

jobs:
  build:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4

      - name: Build
        run: cargo build --release

      - name: Package binary
        run: |
          # Linux: AppImage / tar.gz
          # Windows: zip (include deps)
          # macOS: dmg / tar.gz

      - name: Upload to Release
        uses: softprops/action-gh-release@v2
        with:
          files: runvy-engine-*
```

### Self-Contained Builds

A build is self-contained when it includes all runtime dependencies and
carries a visible version string.

**Current approach** — workspace Cargo.toml version + `git describe`:

```rust
pub fn engine_version() -> &'static str {
    option_env!("VERGEN_GIT_DESCRIBE")
        .unwrap_or(env!("CARGO_PKG_VERSION"))
}
```

**Better: embed version at build time with `vergen` or `built`:**

```toml
[dependencies]
vergen = { version = "9", features = ["build", "git", "cargo"] }
```

```rust
use vergen::EmitBuilder;

fn main() {
    EmitBuilder::builder()
        .all_build()
        .all_git()
        .all_cargo()
        .emit()
        .unwrap();
}
```

Then `env!("VERGEN_BUILD_DATE")`, `env!("VERGEN_GIT_SHA")`, etc. available
in Rust code.

**For packaging:**

```
Linux: cargo build --release → strip → tar.gz
       (consider AppImage for GUI distribution)

Windows: cargo build --release → windeployqt (if Qt) or just zip binary
         (use .exe.manifest for DPI awareness)

macOS: cargo build --release → create-dmg or .app bundle
```

---

## Version Control Workflow

### Branch Model: Feature Branches

```
main ← integration branch, always releasable
  ├── feature/3d-render-wgpu-29   ← new feature
  ├── feature/editor-offscreen     ← new feature
  └── fix/tilemap-culling          ← bug fix
```

**Rules:**

- `main` must always compile (`cargo check`) and pass tests
- Feature branches branch from `main`, merge back via PR
- PRs require CI green + at least one review
- Use squash merge to keep history clean

### Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
feat: add frustum culling for 3D objects
fix: tilemap not rendering after camera move
docs: add rendering-performance.md
refactor: move per-frame Vecs to Renderer struct
perf: replace linear scan with HashMap for texture grouping
ci: add release workflow with cross-platform builds
```

Types: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `ci`, `chore`.

### Changelog

Maintain `CHANGELOG.md` per [Keep a Changelog](https://keepachangelog.com/).
Group changes under:

```
## [Unreleased]

### Added
### Changed
### Fixed
### Removed
### Performance
```

---

## How Other Teams Do It

| Project        | Version Scheme     | Build                    | CI                      | Release                     |
| -------------- | ------------------ | ------------------------ | ----------------------- | --------------------------- |
| **Bevy**       | semver + rc        | `cargo build` + examples | GitHub Actions (matrix) | GitHub Releases + crates.io |
| **Fyrox**      | semver             | `cargo build`            | GitHub Actions          | GitHub Releases + crates.io |
| **Godot-Rust** | semver + git       | `cargo build` + bindings | GitHub Actions          | GitHub Releases + docs      |
| **Rapier**     | semver             | `cargo build`            | GitHub Actions          | crates.io only              |
| **Unity**      | year.release.patch | Custom build system      | Jenkins/DevOps          | Installer per platform      |

**Common patterns:**

1. **Workspace-level version** — one version for all crates, bump together
2. **CI matrix** — Linux + Windows + macOS in one workflow
3. **Automatic changelog** — conventional commits → generated changelog
4. **Release automation** — tag push triggers build + upload
5. **Version display** — `--version` flag in CLI tools, About dialog in GUI

---

## Recommended Tools

| Tool               | Purpose                                             |
| ------------------ | --------------------------------------------------- |
| `cargo-release`    | Automate version bump, tag, publish                 |
| `cargo-bump`       | Bump version in Cargo.toml                          |
| `git-cliff`        | Generate changelog from conventional commits        |
| `cargo-dist`       | Build and package Rust binaries for distribution    |
| `release-plz`      | Automated releases with PR-based version bumps      |
| `vergen` / `built` | Embed git hash, build time, rustc version in binary |

### Quick Start with cargo-release

```toml
# .cargo/release.toml
[workspace]
sign-commit = true
sign-tag = true
push-remote = "origin"
pre-release-commit-message = "chore: release {{version}}"
tag-message = "{{version}}"
```

```bash
cargo install cargo-release
cargo release patch --execute   # bumps 0.5.1 → 0.5.2
cargo release alpha --execute   # bumps 0.6.0-alpha.1 → 0.6.0-alpha.2
```
