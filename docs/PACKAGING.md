# Native packaging for Debian-based and Arch-based Linux

### Native packages: Kali, Debian, Ubuntu and Arch derivatives

**Kali / Debian / Ubuntu**: GitHub Releases publish architecture-specific
`.deb` files for **amd64** and **arm64** (same Debian package across these
distributions). Download the correct architecture and install it with:

```bash
sudo apt install ./whydown_0.12.2-1_amd64.deb  # x86_64 Intel/AMD
# or: sudo apt install ./whydown_0.12.2-1_arm64.deb  # 64-bit ARM
whydown --version
whydown gui
```

The package installs `/usr/bin/whydown`, a freedesktop launcher and icon.
The CLI/TUI work without Python/Qt; `python3` and `python3-pyqt6`
are **recommended** for the desktop GUI. These .deb files are built from
statically linked musl binaries for broad glibc distribution compatibility.
They do not modify any services.

**Arch / Manjaro / CachyOS**: the same AUR `PKGBUILD` works on all three.
Find the ready-to-submit stable source recipe in
[`packaging/aur/whydown/`](../packaging/aur/whydown/) and deployment instructions
in [AUR packaging](../packaging/aur/README.md). A `whydown-bin` recipe may also
be supplied with pinned checksums once the corresponding release archives exist.
AUR submissions are independent of GitHub Releases and require AUR SSH access.
Do not install both the source and binary variants.

**Manual .deb build** (on compatible Debian-family amd64 or arm64):

```bash
cargo build --release --locked
bash packaging/deb/build-deb.sh target/release/whydown dist
dpkg-deb --info dist/whydown_*.deb
```

All native-package scripts are in the repository; the binary releases
remain available for people who prefer the unprivileged `~/.local/bin` installer.



## Release build and AUR validation

The automated release builds both native amd64 and arm64 static musl binaries and generates verified .deb assets. For the AUR, run `makepkg --printsrcinfo > .SRCINFO` under Arch/Manjaro/CachyOS and verify the resulting metadata before publishing. Building from the tagged Rust source requires the original GitHub vX.Y.Z tag.
