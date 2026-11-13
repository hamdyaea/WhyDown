# Publish WhyDown to AUR (Arch, Manjaro, CachyOS)

AUR packages contain **build recipes**, not .deb files or compiled binaries.
The same two recipes work on Arch Linux, Manjaro and CachyOS:

| Package | Recipe | What it installs |
| --- | --- | --- |
| `whydown` | [PKGBUILD](whydown/PKGBUILD) · [.SRCINFO](whydown/.SRCINFO) | Build WhyDown v0.12.3 from tagged Rust source |
| `whydown-bin` | [PKGBUILD](whydown-bin/PKGBUILD) · [.SRCINFO](whydown-bin/.SRCINFO) | Download pinned, verified v0.12.3 static musl binaries |

Only install **one** of these packages: they provide the same executable.
`python-pyqt6` is optional for the CLI and TUI but required for the Qt GUI.

## 1. Prepare AUR account and SSH key (one time)

On Manjaro, CachyOS or Arch:

```bash
sudo pacman -S --needed base-devel git openssh
mkdir -p ~/.ssh
chmod 700 ~/.ssh
ssh-keygen -t ed25519 -a 64 -f ~/.ssh/aur_ed25519 -C "AUR WhyDown"
cat ~/.ssh/aur_ed25519.pub
```

Sign up at https://aur.archlinux.org/ and paste the **public key only**
(`aur_ed25519.pub`) into *My Account → SSH Public Key*.
Never upload `aur_ed25519` (the private key).

Add this host entry to `~/.ssh/config` if one does not already exist:

```sshconfig
Host aur.archlinux.org
  HostName aur.archlinux.org
  User aur
  IdentityFile ~/.ssh/aur_ed25519
  IdentitiesOnly yes
```

Then run `chmod 600 ~/.ssh/config` and test `ssh aur@aur.archlinux.org help`.
Verify the server's SSH host key against an official trusted fingerprint before
accepting it the first time.

## 2. Check package names and prepare source files

Before creating AUR repositories, search https://aur.archlinux.org/ for
`whydown` and `whydown-bin`; **do not overwrite someone else's package or
create a duplicate**. The initial clone of an unregistered package may warn
that you cloned an empty repository; that is expected.

```bash
git clone https://github.com/hamdyaea/WhyDown.git ~/WhyDown-aur-source
mkdir -p ~/aur-publish
```

If `~/WhyDown-aur-source` already exists, use
`git -C ~/WhyDown-aur-source pull --ff-only` instead of cloning again.

## 3. Publish the source-build package `whydown`

```bash
cd ~/aur-publish
git -c init.defaultBranch=master clone ssh://aur@aur.archlinux.org/whydown.git
cp ~/WhyDown-aur-source/packaging/aur/whydown/PKGBUILD ~/aur-publish/whydown/
cd ~/aur-publish/whydown
makepkg --printsrcinfo > .SRCINFO
makepkg --verifysource
git add PKGBUILD .SRCINFO
git commit -m "Publish WhyDown 0.12.3 source build"
git push origin master
```

## 4. Publish the prebuilt `whydown-bin` package

```bash
cd ~/aur-publish
git -c init.defaultBranch=master clone ssh://aur@aur.archlinux.org/whydown-bin.git
cp ~/WhyDown-aur-source/packaging/aur/whydown-bin/PKGBUILD ~/aur-publish/whydown-bin/
cd ~/aur-publish/whydown-bin
makepkg --printsrcinfo > .SRCINFO
makepkg --verifysource
git add PKGBUILD .SRCINFO
git commit -m "Publish WhyDown 0.12.3 prebuilt binary"
git push origin master
```

AUR accepts pushes **only to `master`**. If a package already has Git history
you maintain, clone it and preserve its history before committing updates.
Make sure your `git config user.name` and `git config user.email` in each
package repository are suitable for public commit metadata. The AUR package
becomes visible after the first successful push.

You can run `makepkg -si` inside **one** recipe directory to test the local
installation, then `whydown --version` and `whydown gui`.
Never run `makepkg` as root.

## 5. Alternative publishing workflows

There is **no supported web form to upload a ZIP or .deb directly to AUR**.
The official publishing interface is Git over SSH, regardless of whether the
source files were prepared on GitHub or downloaded as a ZIP.

- **`aurpublish`** is an optional AUR management helper: it automates
  package-specific commits/pushes, but still uses your AUR SSH key. See
  https://man.archlinux.org/man/aurpublish.1.
- **GitHub Actions** can perform these Git/SSH pushes after you configure a
  dedicated AUR deploy private key as an encrypted repository secret and limit
  the workflow to manual dispatch. Never commit the private key to GitHub.

When updating a recipe, bump `pkgver` (or `pkgrel` for packaging-only
changes), regenerate `.SRCINFO` with `makepkg --printsrcinfo`, review the
diff, commit, and push. Keep binaries, `src/`, `pkg/` and generated
`*.pkg.tar.zst` files out of the AUR Git repositories.

Official guidance: https://wiki.archlinux.org/title/AUR_submission_guidelines
