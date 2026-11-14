# Local setup: Claude Code sessions on Mikey's Fedora laptop

Status: 2026-11-14. Mikey runs Hands and Brain sessions in Claude Code
in a terminal. Claude Code does all the work; Mikey gives the
instructions. Fedora 45 KDE Plasma.

## Disk space

About 6 GB in total:
- Rust toolchain: about 1.3 GB (`~/.rustup`, `~/.cargo`)
- the build folder `rust/target/`: about 3 to 4 GB. `cargo clean` in
  `rust/` frees it; the next build takes some minutes again.
- Godot 4.7.1 in `tools/.cache/`: about 150 MB
- Terrain3D in `game/addons/`: about 50 MB
- one game build in `~/Games`: about 80 MB

## One-time setup (Mikey types these)

1. Install the system tools:
   ```
   sudo dnf install -y git gcc curl unzip gh
   ```
2. Install Rust and the pinned version (from `rust/rust-toolchain.toml`):
   ```
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain none
   source ~/.cargo/env
   rustup toolchain install 1.97.0 --component rustfmt --component clippy
   ```
3. Log in to GitHub, so that sessions can push to `main`:
   ```
   gh auth login
   gh auth setup-git
   ```
   Choose GitHub.com, HTTPS, and "Login with a web browser."
4. Get the repo:
   ```
   mkdir -p ~/dev && cd ~/dev
   gh repo clone nowmichaelclark/brokenfriends
   cd brokenfriends
   git config user.name "nowmichaelclark"
   git config user.email "nowmichaelclark@users.noreply.github.com"
   ```
   [unverified: the noreply address; GitHub shows the exact one under
   Settings, Emails.]
5. Run the full check one time. It downloads Godot 4.7.1 and Terrain3D
   and builds everything (about 10 minutes the first time):
   ```
   tools/check.sh
   ```
6. Get the newest game build to play:
   ```
   tools/get-build.sh
   ```

## Each session

1. Open a terminal in the repo:
   ```
   cd ~/dev/brokenfriends
   git pull
   claude
   ```
2. In Claude Code, select the model and the effort that the agenda row
   gives (`plan/STATE.md`, column "Model, effort").
3. Type the row, for example `Run S-136`.
4. When Claude Code asks for permission to run a command, read it and
   decide. If you do not understand a command, ask Claude Code what it
   does before you say yes.
5. One session for each row. Start a new Claude Code session for the
   next row, so the context stays small.

## Playtests

1. `tools/get-build.sh`, then start
   `~/Games/broken-friends-linux/BrokenFriends.x86_64`.
2. Play. Tap Back when something is wrong.
3. Quit. The zip is in `~/Games/broken-friends-linux/playtests/`.
4. In the next session, tell Claude Code: "The playtest is the newest
   zip in ~/Games/broken-friends-linux/playtests/." Claude Code reads it
   there. Do not commit playtests to the repo.
