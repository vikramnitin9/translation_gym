FROM ubuntu:20.04

RUN apt -y update
RUN apt -y upgrade
RUN apt install -y build-essential

ENV DEBIAN_FRONTEND=noninteractive

RUN apt install -y llvm-10 llvm-10-dev llvm-10-tools clang-10 libclang-10-dev
RUN apt install -y cmake
RUN apt install -y wget
RUN apt install -y unzip
RUN apt install -y bear
RUN apt install -y curl

# Install Docker CLI
RUN apt install -y docker.io

RUN apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Add a non-root user with the same UID and GID as the host user
ARG USER_ID
ARG GROUP_ID
RUN if ! getent group ${GROUP_ID}; then \
    groupadd -g ${GROUP_ID} appuser; \
    fi && \
    useradd -m -u ${USER_ID} -g ${GROUP_ID} appuser

RUN mkdir -p /opt/miniconda3 && \
    chown -R ${USER_ID}:{GROUP_ID} /opt/miniconda3

# Set the non-root user as the owner of the /app directory
RUN mkdir -p /app && \
    chown -R ${USER_ID}:{GROUP_ID} /app

# Switch to the non-root user
USER appuser

WORKDIR /app

ENV PATH="/opt/miniconda3/bin:${PATH}"
ARG PATH="/opt/miniconda3/bin:${PATH}"

# Install Miniconda on x86 or ARM platforms
RUN arch=$(uname -m) && \
    if [ "$arch" = "x86_64" ]; then \
    MINICONDA_URL="https://repo.anaconda.com/miniconda/Miniconda3-latest-Linux-x86_64.sh"; \
    elif [ "$arch" = "aarch64" ]; then \
    MINICONDA_URL="https://repo.anaconda.com/miniconda/Miniconda3-latest-Linux-aarch64.sh"; \
    else \
    echo "Unsupported architecture: $arch"; \
    exit 1; \
    fi && \
    wget $MINICONDA_URL -O miniconda.sh && \
    mkdir -p /opt/miniconda3 && \
    bash miniconda.sh -b -u -p /opt/miniconda3 && \
    rm -f miniconda.sh

# Install Rust
ENV CARGO_HOME="/app/.cargo"
ENV RUSTUP_HOME="/app/.rustup"
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/app/.cargo/bin:${PATH}"
RUN rustup install nightly-2024-08-07-x86_64-unknown-linux-gnu

# Ensure the copied files are owned by the non-root user
COPY --chown=${USER_ID}:${GROUP_ID} requirements.txt .
COPY --chown=${USER_ID}:${GROUP_ID} parsec/ parsec/
COPY --chown=${USER_ID}:${GROUP_ID} rust_wrapper rust_wrapper/

RUN pip install -r requirements.txt
RUN cd parsec && \
    rm -rf build && \
    mkdir build && \
    cd build && \
    cmake .. && \
    make -j 4

ENV PARSEC_BUILD_DIR=/app/parsec/build

USER appuser
