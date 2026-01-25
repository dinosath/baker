# See here for image contents: https://github.com/devcontainers/images/tree/main/src/rust

FROM mcr.microsoft.com/devcontainers/rust:latest

# Install additional tools useful for development
RUN apt-get update && apt-get install -y \
    git \
    curl \
    wget \
    tree \
    && rm -rf /var/lib/apt/lists/*
