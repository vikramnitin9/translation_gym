#!/bin/bash
rm -rf /tmp/coreutils
git clone https://git.savannah.gnu.org/git/coreutils /tmp/coreutils
cd /tmp/coreutils
git submodule foreach git pull origin master
git commit -m 'build: update gnulib submodule to latest' gnulib
./bootstrap
./configure --quiet #[--disable-gcc-warnings] [*]
make
make clean
