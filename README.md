# alea

*"alea iacta est"* — The die is cast.

A verifiable random selection tool that uses [drand](https://drand.love)'s
distributed randomness beacon to make choices. Because deciding what to have
for lunch deserves the same cryptographic rigor as a national lottery.

## Why

Sometimes you need to pick between pizza and sushi. You could flip a coin
like a normal person, or you could query a decentralized network of nodes
running threshold BLS signatures on a chained randomness scheme to produce
publicly verifiable, unbiasable entropy — and use that to pick your lunch.

This is that second thing.

Every selection is tied to a specific drand round, meaning anyone can
independently verify the result wasn't tampered with. No trust required.
Just math.

## Install

**Download** — a single binary that runs on Linux, macOS, Windows, FreeBSD,
OpenBSD and NetBSD, on x86-64 and ARM64:

```sh
curl -L https://github.com/hermo/alea/releases/latest/download/alea.ape -o alea
chmod +x alea
sudo mv alea /usr/local/bin/
```

**Homebrew** (macOS and Linux):

```sh
brew install hermo/tap/alea
```

**Build from source:**

```sh
make
sudo make install
```

Requires `libcurl` and OpenSSL `libcrypto` headers:
- macOS: `brew install openssl` (libcurl is already there)
- Debian/Ubuntu: `apt install libcurl4-openssl-dev libssl-dev`

**Build the portable APE binary yourself:**

```sh
curl -L https://cosmo.zip/pub/cosmocc/cosmocc-4.0.2.zip -o /tmp/cosmocc.zip
unzip /tmp/cosmocc.zip -d ~/cosmocc
make ape COSMOCC=~/cosmocc/bin/cosmocc
```

Produces `alea.ape` — runs everywhere without modification.

## Usage

```sh
alea pizza sushi tacos ramen
```

```
🎲 tacos

round: 6100001
time:  2026-05-10T16:38:00Z

verify:
  alea --round 6100001 pizza sushi tacos ramen
```

### Verify a past result

```sh
alea --round 6100001 pizza sushi tacos ramen
```

Same round + same options = same result. Always. Anyone can check.

### Pick multiple winners

Use `--count N` to pick N unique winners in ranked order. No duplicates —
each winner is removed from the pool before the next is drawn:

```sh
alea --count 3 Alice Bob Charlie Dave Eve
```

Maximum 8 winners per draw (8 × 32-bit chunks from the 256-bit drand
value).

### Read options from a file

```sh
alea --file restaurants.txt
alea --file menu.csv --delimiter ","
alea --count 3 --file participants.txt
```

Use `-` to read from stdin:

```sh
git log --format="%an" | sort -u | alea --file -
```

When using `--file`, the output includes a SHA-256 hash of the input so
you can prove the options weren't modified after the fact.

### Schedule a future draw

Pre-calculate the drand round for a specific time. Useful for raffles
where you want to announce the parameters in advance:

```sh
alea --at '2026-07-22T12:00:00+03:00' --count 3 --file participants.txt
```

Share the round number, pick count, and file hash beforehand. When the
time comes, run the command and everyone can verify the result.

### Quiet mode

```sh
winner=$(alea --quiet pizza sushi tacos)
```

### Machine-readable output

```sh
alea --json pizza sushi tacos
alea --tsv pizza sushi tacos | grep '^winner' | cut -f2
```

### Verification oneliners

Don't have `alea` installed? Each shell mode gives you a self-contained
verification command that only needs `curl`:

```sh
alea --sh pizza sushi tacos         # bash/zsh
alea --fish pizza sushi tacos       # fish
alea --ps pizza sushi tacos         # PowerShell
alea --all pizza sushi tacos        # all of the above
```

Pass `--quiet` to get only the raw oneliner, ready to pipe:

```sh
alea --sh --quiet pizza sushi tacos | bash
```

## How it works

1. Fetch a randomness round from drand's public HTTP API
2. Take the first 8 hex characters of the randomness (32 bits)
3. Compute `index = value % option_count`
4. That's your winner

For multiple winners, each pick uses the next 8 hex characters as its
seed, and the chosen option is removed from the pool before the next draw.

The drand network produces a new random value every 30 seconds. Each round
is publicly verifiable and cannot be predicted or biased by any single
party.

## Options

```
--round <N>           Use a specific drand round (for verification)
--at <TIMESTAMP>      Calculate round for a future time (ISO 8601)
-f, --file <path>     Read options from a file
-d, --delimiter <str> Split file by delimiter (default: newline)
-n, --count <N>       Pick N unique winners (default: 1, max: 8)
-q, --quiet           Print only the result, no headers or labels
--all                 Show all verification methods
--json                Machine-readable JSON output
--tsv                 Tab-separated key/value output
--sh                  Output bash/zsh verification oneliner
--fish                Output fish verification oneliner
--ps                  Output PowerShell verification oneliner
-V, --version         Show version
-h, --help            Show this help
```

## Why C

No dependencies to rot. libcurl's ABI has been stable since 2006. This
binary will build unchanged in 30 years.

The portable download is built with [Cosmopolitan libc](https://justine.lol/cosmopolitan/)
and bundles [BearSSL](https://bearssl.org/) for TLS — no runtime dependencies
of any kind. One binary, every platform.

## License

GPL-2.0
