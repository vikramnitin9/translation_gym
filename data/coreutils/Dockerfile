FROM ubuntu:20.04 AS coreutils

RUN apt -y update
RUN apt -y upgrade
RUN apt install -y build-essential

ENV DEBIAN_FRONTEND=noninteractive

RUN apt install -y cmake
RUN apt install -y gdb
RUN apt install -y wget
RUN apt install -y unzip
RUN apt install -y curl
RUN apt install -y python3

# Test dependencies
RUN apt install -y autoconf automake autopoint bison gettext git gperf gzip help2man m4 perl tar texinfo xz-utils

# These two are also needed for the tests
RUN apt install -y libssl-dev
RUN apt install -y libgmp-dev

# Add a non-root user with the same UID and GID as the host user
ARG USER_ID
ARG GROUP_ID
RUN if ! getent group ${GROUP_ID}; then \
    groupadd -g ${GROUP_ID} appuser; \
    fi && \
    useradd -m -u ${USER_ID} -g ${GROUP_ID} appuser

RUN apt install -y sudo
# Create passwordless sudo access for the appuser
RUN echo "${USER_ID} ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers
RUN echo "appuser ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

RUN ["git", "clone", "https://git.savannah.gnu.org/git/coreutils.git", "/app"]

WORKDIR /app

COPY --chown=${USER_ID}:${GROUP_ID} coreutils/patch_tests.sh /app
RUN ["bash", "patch_tests.sh", "/app/tests"]

# Create necessary directories
RUN chown -R ${USER_ID}:${GROUP_ID} /app
RUN mkdir -p /app/instrumentation && chown -R ${USER_ID}:${GROUP_ID} /app/instrumentation
RUN mkdir -p /executable && chown -R ${USER_ID}:${GROUP_ID} /executable

USER appuser

RUN ["make", "-f", "cfg.mk"]

ENV PATH="/executable:${PATH}"
ENV INSTRUMENTATION_PATH="/app/instrumentation"