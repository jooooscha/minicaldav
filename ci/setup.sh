#!/bin/sh

pacman-key --init
pacman -Sy
pacman -S cocogitto base-devel gtk4 libadwaita just openssh --noconfirm
curl https://sh.rustup.rs -sSf | sh -s -- -y