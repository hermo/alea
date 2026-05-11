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

Requires `libcurl` headers at build time (`libcurl4-openssl-dev` on Debian/Ubuntu, `curl` on macOS via Xcode).

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

Use `--count N` (or `-n N`) to pick N unique winners in ranked order. No duplicates — each
winner is removed from the pool before the next is drawn:

```sh
alea --count 3 Alice Bob Charlie Dave Eve
```

```
🎲 1. Charlie
   2. Alice
   3. Eve

round: 6100001
time:  2026-05-10T16:38:00Z

verify:
  alea --count 3 --round 6100001 Alice Bob Charlie Dave Eve
```

Maximum 8 winners per draw (8 × 32-bit chunks from the 256-bit drand value).

### Read options from a file

```sh
alea --file restaurants.txt
alea --file menu.csv --delimiter ","
alea --count 3 --file participants.txt
```

When using `--file`, the output includes a SHA-256 hash of the input file so you can prove the options weren't modified after the fact.

### Schedule a future draw

Pre-calculate the drand round for a specific time. Useful for raffles where you want to announce the parameters in advance:

```sh
alea --at '2026-07-22T12:00:00+03:00' --count 3 --file participants.txt
```

```
Scheduled alea run:

round: 6309325
time:  2026-07-22T09:00:00Z
input: sha256:724f60bb74a1302049595da515add3092cffb0acec5649462e8d1d279d1ffd4d
count: 42 options
picks: 3

run at the scheduled time:
  alea --round 6309325 --count 3 --file participants.txt
```

Share the round number, pick count, and file hash beforehand. When the time comes, run the command and everyone can verify the result.

If the timestamp is already in the past, `--at` shows a historical run instead, with the same verify command:

```
Historical alea run:
...
run now:
  alea --round 240162 --file participants.txt
```

### Quiet mode

Print only the winner(s), no headers or labels — useful for scripting:

```sh
alea --quiet pizza sushi tacos
```

```
pizza
```

With `--count`, each winner is printed on its own line:

```sh
alea --quiet --count 2 pizza sushi tacos ramen
```

```
tacos
pizza
```

### Machine-readable output

```sh
# JSON (for scripts, jq, etc.)
alea --json pizza sushi tacos
alea --count 3 --json pizza sushi tacos ramen burger

# TSV (for grep, awk, cut)
alea --tsv pizza sushi tacos | grep '^winner' | cut -f2
```

JSON output always includes a `winners` array. For single-winner draws, `winner` and `index`
keys are also present for backward compatibility.

### Verification oneliners

Don't have `alea` installed? Each output mode includes a self-contained verification command that only needs `curl`:

```sh
alea --sh pizza sushi tacos
```

```
🎲 pizza

round: 6100003
time:  2026-05-10T16:39:00Z

verify (bash/zsh):
  # alea pizza sushi tacos --round 6100003 => pizza
  opts=(pizza sushi tacos); r=$(curl -s https://api.drand.sh/public/6100003 | grep -o '"randomness":"[^"]*"' | cut -d'"' -f4); i=$(printf "%d" "0x${r:0:8}"); echo "${opts[$((i % ${#opts[@]}))]}"
```

Use `--fish` or `--ps` for the equivalent fish or PowerShell command. Use `--all` to show all variants at once.

Pass `--quiet` to get only the raw oneliner with no headers, ready to pipe or run directly:

```sh
alea --sh --quiet pizza sushi tacos | bash
```

Shell oneliners are only available for single-winner draws (`--count 1`).

## How it works

1. Fetch a randomness round from drand's public HTTP API
2. Take the first 8 hex characters of the randomness (32 bits)
3. Compute `index = value % option_count`
4. That's your winner

For multiple winners, each pick uses the next 8 hex characters as its seed, and the chosen
option is removed from the pool before the next draw.

The drand network produces a new random value every 30 seconds. Each round is publicly verifiable and cannot be predicted or biased by any single party.

## Options

```
Usage: alea [OPTIONS] <option1> <option2> [option3...]
       alea [OPTIONS] --file <path> [--delimiter <delim>]

Options:
  --round <N>           Use a specific drand round (for verification)
  --at <TIMESTAMP>      Calculate round for a future time (ISO 8601)
  -f, --file <path>     Read options from a file
  -d, --delimiter <str> Split file by delimiter (default: newline)
  -n, --count <N>       Pick N unique winners (default: 1, max: 8)
  -q, --quiet           Print only the result, no headers or labels
  --all                 Show all verification methods
  --json                Machine-readable JSON output
  --tsv                 Tab-separated key/value output (grep/awk/cut friendly)
  --sh                  Output bash/zsh verification oneliner (single winner only)
  --fish                Output fish verification oneliner (single winner only)
  --ps                  Output PowerShell verification oneliner (single winner only)
  -V, --version         Show version
  -h, --help            Show this help
```

## License

GPL-2.0
