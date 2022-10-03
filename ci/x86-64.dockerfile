FROM manjarolinux/base:20220907
RUN pacman -Syu base-devel wget git --noconfirm && \
    curl https://sh.rustup.rs -sSf | sh -s -- -y && \
    source $HOME/.cargo/env && \
    cargo install cocogitto --locked && \
    cargo install cargo-outdated --locked && \
    cargo install cargo-bump --locked && \
    cargo install cargo-gra --locked && \
    cargo install cargo-audit --locked && \
    paccache -rvk0
