# alea

*"alea iacta est"* - The die is cast.

A verifiable random selection tool that uses [drand](https://drand.love)'s distributed randomness beacon to make choices. Because deciding what to have for lunch deserves the same cryptographic rigor as a national lottery.

## Why

Sometimes you need to pick between pizza and sushi. You could flip a coin like a normal person, or you could query a decentralized network of nodes running threshold BLS signatures on a chained randomness scheme to produce publicly verifiable, unbiasable entropy - and use that to pick your lunch.

This is that second thing.

Every selection is tied to a specific drand round, meaning anyone can independently verify the result wasn't tampered with. No trust required. Just math.

## Install

### Homebrew (macOS Apple Silicon, Linux x86_64)

```sh
brew tap hermo/tap
brew install alea
```

### Shell installer

```sh
curl -sSf https://raw.githubusercontent.com/hermo/alea/main/install.sh | sh
```

### From source

```sh
cargo install --path .
```

## Usage

```sh
# Pick lunch
alea pizza sushi tacos ramen

# The output includes a verification command
🎲 tacos

round: 6102380
time:  2026-05-11T12:27:30Z

verify:
  alea --round 6102380 pizza sushi tacos ramen
```

### Verify a past result

```sh
alea --round 6102380 pizza sushi tacos ramen
```

Same round + same options = same result. Always. Anyone can check.

### Read options from a file

```sh
alea --file restaurants.txt
alea --file menu.csv --delimiter ","
```

When using `--file`, the output includes a SHA-256 hash of the input file so you can prove the options weren't modified after the fact.

### Machine-readable output

```sh
# JSON (for scripts, jq, etc.)
alea --json pizza sushi tacos

# TSV (for grep, awk, cut)
alea --tsv pizza sushi tacos | grep '^winner' | cut -f2
```

### Verification oneliners

Don't have `alea` installed? Get a self-contained verification command:

```sh
# bash/zsh
alea --sh pizza sushi tacos

# fish
alea --fish pizza sushi tacos

# PowerShell
alea --ps pizza sushi tacos
```

These output a single command that only needs `curl` (or `Invoke-RestMethod`) to verify the result independently.

### Show all verification methods at once

```sh
alea --all pizza sushi tacos
```

## How it works

1. Fetch a randomness round from drand's public HTTP API
2. Take the first 8 hex characters of the randomness (32 bits)
3. Compute `index = value % option_count`
4. That's your winner

The drand network produces a new random value every 30 seconds. Each round is publicly verifiable and cannot be predicted or biased by any single party.

## Options

```
Usage: alea [OPTIONS] <option1> <option2> [option3...]
       alea [OPTIONS] --file <path> [--delimiter <delim>]

Options:
  --round <N>       Use a specific drand round (for verification)
  -f, --file <path> Read options from a file
  -d, --delimiter <str> Split file by delimiter (default: newline)
  --all             Show all verification methods
  --json            Machine-readable JSON output
  --tsv             Tab-separated key/value output (grep/awk/cut friendly)
  --sh              Output bash/zsh verification oneliner
  --fish            Output fish verification oneliner
  --ps              Output PowerShell verification oneliner
  -h, --help        Show this help
```

## License

GPL-2.0
