# A fleet INSTANCE: fleet's whole runtime in one image, ready to be used.
#
# This is not the desktop app and not a second product. It is the same binary
# that runs on a laptop, built for linux, plus everything it reaches for at
# run time. Armada deploys it (`fleet armada deploy --provider docker|openship`)
# and the container joins a fleet as an instance: it inherits the shared
# corpus, registers on the flagship's roster, and heartbeats.
#
# Why multi-stage: the crate links `ring`, which cross-compiles badly from
# darwin. Building INSIDE a linux stage is the boring path — no cross
# toolchain, no target triple to pin, and the binary is tested against the
# same libc it will run on.
#
# The image carries NO secrets. Credentials arrive as env or through the
# vault on the mounted volume; a layer here holds public code only.

# --------------------------------------------------------------------------
# Stage 1 — build the binary on linux.
# --------------------------------------------------------------------------
FROM rust:1-bookworm AS build

# What the BUILD needs beyond rustc, measured from a build that failed without
# it rather than guessed: a C toolchain for `ring`'s assembly, `pkg-config` for
# the -sys crates, and `libasound2-dev` because `cpal` (the composer's
# dictation) builds `alsa-sys` even when no microphone will ever exist in a
# container. The alsa LIBRARY is not installed at runtime — an instance does
# not record — but the -sys crate needs its headers to compile.
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      build-essential pkg-config libasound2-dev \
 && rm -rf /var/lib/apt/lists/*

WORKDIR /src
# The whole tree, then one build. The dependency-caching stub trick is
# deliberately not used here: it needs a fake `main.rs`/`lib.rs` to satisfy
# `cargo`, and `-p fleet` builds BOTH targets, so the stub would have to
# impersonate the real lib to compile — a trick that buys a cache hit and
# costs a build that can silently compile the wrong source.
COPY . .
# `-p fleet` and NOT the workspace: `default-members` includes `src-tauri`,
# and the desktop shell links GTK, which an instance neither needs nor could
# install in a slim runtime. An instance is the CLI and its service lanes.
RUN cargo build --release --locked -p fleet

# --------------------------------------------------------------------------
# Stage 2 — the runtime.
# --------------------------------------------------------------------------
FROM debian:bookworm-slim

# What the running binary needs, measured by RUNNING it rather than guessed
# from the tree: the first build booted to "error while loading shared
# libraries: libasound.so.2", because `cpal` links the audio library even
# though an instance never records. `ca-certificates` for the provider APIs
# and the release feed, `git` for the corpus (33 call sites), `python3` for
# the document lane and the venv-built service lanes, `openssh-client` for a
# future ssh provider and for git remotes over ssh, `tini` to reap the
# children fleet spawns (a bare PID 1 does not).
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      ca-certificates git python3 python3-venv openssh-client tini \
      libasound2 libstdc++6 \
 && rm -rf /var/lib/apt/lists/*

# An unprivileged user owns the process. The instance needs nothing the
# root user has, and the image says so rather than implying otherwise.
RUN useradd --create-home --uid 10001 --shell /usr/sbin/nologin fleet

# The binary and the service lanes beside it: `lanes::services_root()` looks
# next to the executable before it walks, so `/opt/fleet/services` resolves
# with no environment at all.
COPY --from=build /src/target/release/fleet /opt/fleet/fleet
COPY services /opt/fleet/services
# The bundled skills the prompt assembles from.
COPY --from=build /src/docs /opt/fleet/docs

# The skills library lives at `$FLEET_SKILLS_DIR`, else `<user home>/.agents/
# skills`. An instance pins it under the VOLUME, so the shared intelligence
# survives a container replacement exactly as the corpus does — a redeploy
# that re-cloned 118 MB of skills every time would be both slow and pointless.
ENV PATH=/opt/fleet:$PATH \
    FLEET_HOME=/data \
    FLEET_SERVICES_ROOT=/opt/fleet/services \
    FLEET_SKILLS_DIR=/data/skills-canonical

# All state lives here: sessions, the corpus, grants, identity, the roster,
# the journal, the vault. One named volume at /data is the whole
# persistence contract.
RUN mkdir -p /data && chown fleet:fleet /data
VOLUME ["/data"]

USER fleet
WORKDIR /data

# `git` refuses a repository whose owner differs from the caller — which is
# every bind-mounted remote, and many volume layouts. An instance clones from
# a remote an operator mounted, so the ownership check would refuse the very
# corpus the instance exists to inherit. Setting the exception for the mount
# points (not `*`) keeps the protection where it matters — the instance's own
# tree is still checked — while letting an operator hand it a repository.
# The config must land in the FLEET user's home, and `USER` is already
# fleet here, so the call names the file explicitly rather than trusting
# `--global` to resolve `$HOME` the way the shell does.
RUN git config --file /home/fleet/.gitconfig --add safe.directory /corpus.git \
 && git config --file /home/fleet/.gitconfig --add safe.directory /data \
 && git config --file /home/fleet/.gitconfig --add safe.directory /data/memories \
 && git config --file /home/fleet/.gitconfig --add safe.directory /skills.git \
 && git config --file /home/fleet/.gitconfig --add safe.directory /data/skills-canonical \
 && git config --file /home/fleet/.gitconfig --add safe.directory /mcp.git \
 && cat /home/fleet/.gitconfig

# `tini` reaps the children fleet spawns (the service lanes, git, the
# browser engine), which a bare PID 1 does not.
ENTRYPOINT ["/usr/bin/tini", "--", "/opt/fleet/fleet"]

# The default is the instance boot: join a fleet from env, then heartbeat.
# A bare `docker run` with no env refuses by name rather than starting a
# half-configured instance.
CMD ["armada", "join"]
