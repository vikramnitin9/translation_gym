FROM ubuntu:20.04

RUN apt -y update
RUN apt -y upgrade
RUN apt install -y build-essential

ENV DEBIAN_FRONTEND=noninteractive

RUN apt install -y cmake
RUN apt install -y wget
RUN apt install -y unzip
RUN apt install -y curl

# These two are needed for the tests
RUN apt install -y libssl-dev
RUN apt install -y libgmp-dev
RUN apt install -y perl

WORKDIR /app

# Create necessary directories
RUN mkdir -p /app/tests/tail
RUN mkdir -p /app/src/tail
RUN mkdir -p /app/getlimits

# Copy specific test directory
COPY coreutils/tests/tail/ /app/tests/tail/
COPY coreutils/src/tail/ /app/src/tail/

# Copy common test files from tests directory
COPY coreutils/tests/envvar-check /app/tests/envvar-check
COPY coreutils/tests/init.sh /app/tests/init.sh
COPY coreutils/tests/lang-default /app/tests/lang-default
COPY coreutils/tests/setup_test_env.sh /app/tests/setup_test_env.sh

# Copy getlimits directory and init.cfg
COPY coreutils/getlimits/ /app/getlimits/
COPY coreutils/init.cfg /app/init.cfg

ENV TMPDIR=/tmp \
    VERSION=9.5.42-bbc97 \
    RUN_EXPENSIVE_TESTS=yes \
    RUN_VERY_EXPENSIVE_TESTS=yes \
    LOCALE_FR=none \
    LOCALE_FR_UTF8=none \
    abs_top_builddir=/tmp/coreutils \
    abs_top_srcdir=/tmp/coreutils \
    fail=0 \
    host_os=linux-gnu \
    host_triplet=aarch64-unknown-linux-gnu \
    srcdir=/app/tests/ \
    top_srcdir=/app/tests/ \
    CONFIG_HEADER=/app/coreutils/src/tail/c/include/config.h \
    CC='gcc' \
    AWK='mawk' \
    EGREP='/usr/bin/grep -E' \
    EXEEXT='' \
    MAKE='/usr/bin/make' \
    PACKAGE_VERSION=9.5.42-bbc97 \
    PERL='perl' \
    SHELL='/bin/bash'

RUN ["sh", "-c", "tests/setup_test_env.sh"]

ENV PATH="/app/getlimits:/executable:${PATH}"

CMD ["bash", "/app/tests/tail/retry.sh", "&&", \
     "bash", "/app/tests/tail/inotify-only-regular.sh", "&&", \
     "bash", "/app/tests/tail/assert.sh", "&&", \
     "bash", "/app/tests/tail/F-vs-rename.sh", "&&", \
     "bash", "/app/tests/tail/assert-2.sh", "&&", \
     "bash", "/app/tests/tail/inotify-dir-recreate.sh", "&&", \
     "bash", "/app/tests/tail/big-4gb.sh", "&&", \
     "bash", "/app/tests/tail/F-headers.sh", "&&", \
     "bash", "/app/tests/tail/append-only.sh", "&&", \
     "bash", "/app/tests/tail/follow-name.sh", "&&", \
     "bash", "/app/tests/tail/pipe-f.sh", "&&", \
     "bash", "/app/tests/tail/follow-stdin.sh", "&&", \
     "bash", "/app/tests/tail/end-of-device.sh", "&&", \
     "bash", "/app/tests/tail/start-middle.sh", "&&", \
     "bash", "/app/tests/tail/inotify-hash-abuse2.sh", "&&", \
     "bash", "/app/tests/tail/wait.sh", "&&", \
     "bash", "/app/tests/tail/overlay-headers.sh", "&&", \
     "bash", "/app/tests/tail/proc-ksyms.sh", "&&", \
     "bash", "/app/tests/tail/inotify-race.sh"]
