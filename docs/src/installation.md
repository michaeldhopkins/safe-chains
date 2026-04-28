# Installation

## Homebrew (macOS)

```bash
brew install michaeldhopkins/tap/safe-chains
```

## Pre-built binary

Download signed, notarized binaries from [GitHub Releases](https://github.com/michaeldhopkins/safe-chains/releases/latest). Available for macOS (Apple Silicon and Intel) and Linux (x86_64 and aarch64).

```bash
curl -L https://github.com/michaeldhopkins/safe-chains/releases/latest/download/safe-chains-aarch64-apple-darwin.tar.gz | tar xz
mv safe-chains /usr/local/bin/   # or anywhere in your PATH
```

## With Cargo

```bash
cargo install safe-chains
```

## From source

```bash
git clone git@github.com:michaeldhopkins/safe-chains.git
cd safe-chains
cargo install --path .
```
