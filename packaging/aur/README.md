# AUR packaging — Arch Linux / Manjaro / CachyOS

Arch Linux, Manjaro and CachyOS use **the same AUR recipes**; do not create
separate packages for each distribution.

- `whydown/`: stable `whydown` from the tagged Rust source release.
- `whydown-bin/`: stable `whydown-bin` from verified upstream static musl
  x86_64/aarch64 release archives. (Use only after corresponding vX.Y.Z
  release assets have been published; update archive SHA256 with each tag.)

To test on an Arch-derived machine, enter ONE package directory and run
`makepkg --printsrcinfo > .SRCINFO`, `makepkg -si` and
`whydown --version`. Inspect the generated files before installing.

To submit a package for the first time, configure SSH on aur.archlinux.org,
create/clone `ssh://aur@aur.archlinux.org/whydown.git` (or
`whydown-bin.git`), copy only `PKGBUILD` and `.SRCINFO` into that
repository, regenerate `.SRCINFO` using makepkg, then `git add`,
`git commit`, and `git push origin master`.

Do NOT commit compiled binaries, user-local paths or `src/` / `pkg/`
build directories. AUR submission requires authentication and is a
separate action from committing the files to GitHub.

The GUI requires `python-pyqt6`; CLI/TUI need no Qt dependency. Never
install both `whydown` and `whydown-bin` simultaneously.
