# Versioning and releases

`!mouse` uses Semantic Versioning: `MAJOR.MINOR.PATCH`.

During the experimental `0.x` period:

- minor versions may contain breaking changes;
- patch versions contain compatible fixes and small improvements;
- prerelease suffixes such as `-alpha.1` mark builds that are not ready for
  everyday use.

The version in the root Cargo workspace is the source of truth. A release is:

1. a reviewed update to the workspace version and changelog;
2. a signed Git tag named `v<version>`;
3. a GitHub Release created from that tag;
4. packages produced from the exact tagged commit.

Early releases will provide a portable binary archive. Once the daemon and
desktop integration are stable enough to install, releases will also provide:

- a Debian package for Ubuntu and Debian-based systems;
- an RPM package for Fedora and related systems;
- checksums for every downloadable artifact.

Distribution-specific repositories, Flatpak, or AppImage can be added after the
Wayland permission and background-service model has settled. Package versions
must always match the Git tag and embedded application version.

Normal commits to `main` are development builds and do not need unique release
versions. When useful, they can identify themselves with the base version plus
the short Git commit, for example `0.1.0-alpha.1+7dcb1ef`.
