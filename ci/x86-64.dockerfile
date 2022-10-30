FROM manjarolinux/base
RUN pacman -Syu glibc lib32-glibc base-devel wget git --noconfirm && \
    pacman -Scc &&\
    paccache -rvk0 &&\
    curl https://sh.rustup.rs -sSf | sh -s -- -y && \
    source $HOME/.cargo/env && \
    cargo install cocogitto --locked && \
    cargo install cargo-outdated --locked && \
    cargo install cargo-bump --locked && \
    cargo install cargo-audit --locked && \
    rm -rf .cargo/registry
