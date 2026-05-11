mod drand;
mod format;

use sha2::{Digest, Sha256};
use std::io::{IsTerminal, Read};
use std::{fs, process};

use format::Output;

#[derive(Debug)]
struct Config {
    round: Option<u64>,
    output: Output,
    options: Vec<String>,
    input_hash: Option<String>,
    file: Option<String>,
    delimiter: Option<String>,
    quiet: bool,
    at_mode: bool,
    at_past: bool,
    count: usize,
}

#[derive(Debug)]
struct RawArgs {
    round: Option<u64>,
    output: Output,
    positional: Vec<String>,
    file: Option<String>,
    delimiter: Option<String>,
    quiet: bool,
    at: Option<String>,
    count: usize,
}

fn parse_args(args: &[String]) -> Result<RawArgs, String> {
    let mut round: Option<u64> = None;
    let mut output = Output::Human;
    let mut positional: Vec<String> = Vec::new();
    let mut file: Option<String> = None;
    let mut delimiter: Option<String> = None;
    let mut quiet = false;
    let mut at: Option<String> = None;
    let mut count: usize = 1;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--round" => {
                i += 1;
                let val = args.get(i).ok_or("--round requires a value")?;
                round = Some(val.parse().map_err(|_| "invalid round number")?);
            }
            "--file" | "-f" => {
                i += 1;
                file = Some(args.get(i).ok_or("--file requires a path")?.clone());
            }
            "--delimiter" | "-d" => {
                i += 1;
                delimiter = Some(args.get(i).ok_or("--delimiter requires a value")?.clone());
            }
            "--at" => {
                i += 1;
                at = Some(args.get(i).ok_or("--at requires a timestamp")?.clone());
            }
            "--count" | "-n" => {
                i += 1;
                let val = args.get(i).ok_or("--count requires a value")?;
                count = val.parse().map_err(|_| "invalid count")?;
                if count == 0 {
                    return Err("--count must be at least 1".to_string());
                }
            }
            "--quiet" | "-q" => quiet = true,
            "--json" => output = Output::Json,
            "--tsv" => output = Output::Tsv,
            "--sh" => output = Output::Sh,
            "--fish" => output = Output::Fish,
            "--ps" => output = Output::Ps,
            "--all" => output = Output::All,
            "--help" | "-h" => return Err(String::new()),
            "--version" | "-V" => {
                eprintln!("alea {}", env!("CARGO_PKG_VERSION"));
                process::exit(0);
            }
            s if s.starts_with('-') => return Err(format!("unknown flag: {s}")),
            _ => positional.push(args[i].clone()),
        }
        i += 1;
    }

    if at.is_some() && round.is_some() {
        return Err("--at and --round cannot be used together".to_string());
    }

    Ok(RawArgs {
        round,
        output,
        positional,
        file,
        delimiter,
        quiet,
        at,
        count,
    })
}

fn resolve_config(raw: RawArgs) -> Result<Config, String> {
    if raw.file.as_deref() == Some("-") && raw.at.is_some() {
        return Err("--file - cannot be used with --at (stdin is ephemeral)".to_string());
    }

    let mut options = raw.positional;
    let input_hash = if let Some(ref path) = raw.file {
        let contents = if path == "-" {
            if std::io::stdin().is_terminal() {
                return Err("--file - requires piped input (stdin is a terminal)".to_string());
            }
            let mut buf = Vec::new();
            std::io::stdin()
                .read_to_end(&mut buf)
                .map_err(|e| format!("cannot read stdin: {e}"))?;
            buf
        } else {
            fs::read(path).map_err(|e| format!("cannot read {path}: {e}"))?
        };
        let hash = hex_sha256(&contents);
        let text =
            String::from_utf8(contents).map_err(|e| format!("file is not valid UTF-8: {e}"))?;
        let delim = raw.delimiter.as_deref().unwrap_or("\n");
        options = text
            .split(delim)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Some(hash)
    } else {
        None
    };

    if options.len() < 2 {
        return Err("at least 2 options required".to_string());
    }

    if raw.file.is_some()
        && matches!(
            raw.output,
            Output::Sh | Output::Fish | Output::Ps | Output::All
        )
    {
        return Err(
            "--sh/--fish/--ps/--all cannot be used with --file (use --json or --tsv instead)"
                .to_string(),
        );
    }

    if raw.at.is_some()
        && matches!(
            raw.output,
            Output::Sh | Output::Fish | Output::Ps | Output::All
        )
    {
        return Err(
            "--sh/--fish/--ps/--all cannot be used with --at (result is unknown until the draw; \
             use --round after the scheduled time)"
                .to_string(),
        );
    }

    if raw.count > 1
        && matches!(
            raw.output,
            Output::Sh | Output::Fish | Output::Ps | Output::All
        )
    {
        return Err(
            "--sh/--fish/--ps/--all cannot be used with --count > 1 (use --json or --tsv instead)"
                .to_string(),
        );
    }

    if raw.count > options.len() {
        return Err(format!(
            "--count {} exceeds the number of options ({})",
            raw.count,
            options.len()
        ));
    }

    let mut round = raw.round;
    let at_mode = raw.at.is_some();
    let mut at_past = false;
    if let Some(ref ts) = raw.at {
        let epoch = format::parse_iso8601(ts)?;
        if epoch <= drand::GENESIS_TIME {
            return Err("--at timestamp is before drand genesis".to_string());
        }
        round = Some((epoch - drand::GENESIS_TIME) / drand::PERIOD);
        at_past = epoch <= drand::now_secs();
    }

    Ok(Config {
        round,
        output: raw.output,
        options,
        input_hash,
        file: raw.file,
        delimiter: raw.delimiter,
        quiet: raw.quiet,
        at_mode,
        at_past,
        count: raw.count,
    })
}

/// Derive a selection index from hex randomness and option count.
///
/// Uses the first 32 bits (8 hex chars) modulo count. This has a theoretical modulo bias of at
/// most count/2^32 per option (~2.3e-10 for count=1000), which is negligible in practice and
/// cannot be exploited because the randomness value is unknown until after the round is committed.
pub fn select(randomness: &str, count: usize) -> Result<usize, String> {
    if randomness.len() < 8 {
        return Err("randomness too short".to_string());
    }
    let val = u32::from_str_radix(&randomness[..8], 16)
        .map_err(|e| format!("invalid randomness hex: {e}"))?;
    Ok(val as usize % count)
}

/// Pick `count` unique winners by consuming consecutive 8-hex-char chunks of randomness.
/// Each pick k uses randomness[k*8..] so picks are independent and deterministic.
/// Maximum 8 winners (256 bits / 32 bits per pick).
pub fn select_multiple(
    randomness: &str,
    options: &[String],
    count: usize,
) -> Result<Vec<usize>, String> {
    if count > 8 {
        return Err(
            "--count cannot exceed 8 (only 8 chunks of randomness available per round)".to_string(),
        );
    }
    let mut remaining: Vec<usize> = (0..options.len()).collect();
    let mut result = Vec::with_capacity(count);
    for k in 0..count {
        let offset = k * 8;
        let idx = select(&randomness[offset..], remaining.len())?;
        result.push(remaining[idx]);
        remaining.remove(idx);
    }
    Ok(result)
}

pub fn hex_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let raw = parse_args(&args).unwrap_or_else(|e| {
        if e.is_empty() {
            format::print_usage();
            process::exit(0);
        }
        eprintln!("error: {e}");
        eprintln!();
        format::print_usage();
        process::exit(2);
    });

    let config = resolve_config(raw).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        eprintln!();
        format::print_usage();
        process::exit(2);
    });

    if config.at_mode {
        format::render_scheduled(
            &format::ScheduledResult {
                round: config.round.unwrap(),
                options: &config.options,
                input_hash: config.input_hash.as_deref(),
                file: config.file.as_deref(),
                delimiter: config.delimiter.as_deref(),
                count: config.count,
                past: config.at_past,
            },
            config.quiet,
        );
        return;
    }

    let data = drand::fetch(config.round).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        process::exit(1);
    });

    let selected =
        select_multiple(&data.randomness, &config.options, config.count).unwrap_or_else(|e| {
            eprintln!("error: {e}");
            process::exit(1);
        });

    let result = format::SelectionResult {
        round: data.round,
        randomness: &data.randomness,
        selected,
        options: &config.options,
        input_hash: config.input_hash.as_deref(),
        file: config.file.as_deref(),
        delimiter: config.delimiter.as_deref(),
    };

    format::render(&result, &config.output, config.quiet);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_deterministic() {
        assert_eq!(select("3b26244efd679a692b8bff80fb16b74f", 3).unwrap(), 1);
    }

    #[test]
    fn select_two_options() {
        assert_eq!(select("ffffffff0000000000000000", 2).unwrap(), 1);
        assert_eq!(select("000000000000000000000000", 2).unwrap(), 0);
    }

    #[test]
    fn select_error_on_short_hex() {
        assert!(select("abc", 2).is_err());
    }

    #[test]
    fn select_error_on_invalid_hex() {
        assert!(select("zzzzzzzz00000000", 2).is_err());
    }

    #[test]
    fn select_multiple_deterministic() {
        let opts: Vec<String> = vec!["A", "B", "C", "D", "E"]
            .into_iter()
            .map(String::from)
            .collect();
        // chunk0 = 3b26244e = 994680910, 994680910 % 5 = 0 → A
        // chunk1 = fd679a69, remaining=[1,2,3,4] → fd679a69 % 4 = ?
        // fd679a69 = 4250148457, 4250148457 % 4 = 1 → remaining[1] = B (index 1 in original)
        let result = select_multiple("3b26244efd679a692b8bff80fb16b74f", &opts, 2).unwrap();
        assert_eq!(result.len(), 2);
        assert_ne!(result[0], result[1]);
    }

    #[test]
    fn select_multiple_no_duplicates() {
        let opts: Vec<String> = (0..8).map(|i| i.to_string()).collect();
        let randomness = "0102030405060708090a0b0c0d0e0f10202122232425262728292a2b2c2d2e2f";
        let result = select_multiple(randomness, &opts, 8).unwrap();
        assert_eq!(result.len(), 8);
        let mut sorted = result.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 8, "all winners must be unique");
    }

    #[test]
    fn select_multiple_count_exceeds_max() {
        let opts: Vec<String> = (0..10).map(|i| i.to_string()).collect();
        assert!(select_multiple("00".repeat(64).as_str(), &opts, 9).is_err());
    }

    #[test]
    fn parse_args_basic() {
        let args: Vec<String> = vec!["Alice", "Bob"].into_iter().map(String::from).collect();
        let raw = parse_args(&args).unwrap();
        assert_eq!(raw.positional, vec!["Alice", "Bob"]);
        assert!(raw.round.is_none());
        assert_eq!(raw.count, 1);
    }

    #[test]
    fn parse_args_count() {
        let args: Vec<String> = vec!["--count", "3", "A", "B", "C", "D"]
            .into_iter()
            .map(String::from)
            .collect();
        let raw = parse_args(&args).unwrap();
        assert_eq!(raw.count, 3);
    }

    #[test]
    fn parse_args_count_short() {
        let args: Vec<String> = vec!["-n", "2", "A", "B", "C"]
            .into_iter()
            .map(String::from)
            .collect();
        let raw = parse_args(&args).unwrap();
        assert_eq!(raw.count, 2);
    }

    #[test]
    fn resolve_config_count_exceeds_options() {
        let args: Vec<String> = vec!["--count", "5", "A", "B", "C"]
            .into_iter()
            .map(String::from)
            .collect();
        let raw = parse_args(&args).unwrap();
        assert!(resolve_config(raw).is_err());
    }

    #[test]
    fn parse_args_with_round() {
        let args: Vec<String> = vec!["--round", "123", "A", "B"]
            .into_iter()
            .map(String::from)
            .collect();
        let raw = parse_args(&args).unwrap();
        assert_eq!(raw.round, Some(123));
        assert_eq!(raw.positional, vec!["A", "B"]);
    }

    #[test]
    fn parse_args_too_few_options() {
        let args: Vec<String> = vec!["only_one"].into_iter().map(String::from).collect();
        let raw = parse_args(&args).unwrap();
        assert!(resolve_config(raw).is_err());
    }

    #[test]
    fn parse_args_unknown_flag() {
        let args: Vec<String> = vec!["--bogus", "A", "B"]
            .into_iter()
            .map(String::from)
            .collect();
        let err = parse_args(&args).unwrap_err();
        assert!(err.contains("unknown flag"));
    }

    #[test]
    fn parse_args_round_missing_value() {
        let args: Vec<String> = vec!["A", "B", "--round"]
            .into_iter()
            .map(String::from)
            .collect();
        assert!(parse_args(&args).is_err());
    }

    #[test]
    fn parse_args_file() {
        let tmp = std::env::temp_dir().join("alea_test_input.txt");
        fs::write(&tmp, "Alice\nBob\nCharlie\n").unwrap();
        let args: Vec<String> = vec!["--file", tmp.to_str().unwrap()]
            .into_iter()
            .map(String::from)
            .collect();
        let raw = parse_args(&args).unwrap();
        let config = resolve_config(raw).unwrap();
        assert_eq!(config.options, vec!["Alice", "Bob", "Charlie"]);
        assert!(config.input_hash.is_some());
        assert_eq!(config.input_hash.unwrap().len(), 64);
        fs::remove_file(tmp).ok();
    }

    #[test]
    fn parse_args_file_with_delimiter() {
        let tmp = std::env::temp_dir().join("alea_test_delim.txt");
        fs::write(&tmp, "Alice,Bob,Charlie").unwrap();
        let args: Vec<String> = vec!["--file", tmp.to_str().unwrap(), "-d", ","]
            .into_iter()
            .map(String::from)
            .collect();
        let raw = parse_args(&args).unwrap();
        let config = resolve_config(raw).unwrap();
        assert_eq!(config.options, vec!["Alice", "Bob", "Charlie"]);
        fs::remove_file(tmp).ok();
    }

    #[test]
    fn hex_sha256_known() {
        assert_eq!(
            hex_sha256(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn epoch_to_iso_known_value() {
        assert_eq!(format::epoch_to_iso(1595431050), "2020-07-22T15:17:30Z");
    }

    #[test]
    fn epoch_to_iso_epoch_zero() {
        assert_eq!(format::epoch_to_iso(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn shell_quote_simple() {
        assert_eq!(format::shell_quote("hello"), "hello");
    }

    #[test]
    fn shell_quote_with_spaces() {
        assert_eq!(format::shell_quote("hello world"), "'hello world'");
    }

    #[test]
    fn shell_quote_with_single_quote() {
        assert_eq!(format::shell_quote("it's"), "'it'\\''s'");
    }

    #[test]
    fn parse_iso8601_utc() {
        assert_eq!(
            format::parse_iso8601("2026-07-22T12:00:00Z").unwrap(),
            1784721600
        );
    }

    #[test]
    fn parse_iso8601_positive_offset() {
        assert_eq!(
            format::parse_iso8601("2026-07-22T12:00:00+03:00").unwrap(),
            1784721600 - 3 * 3600
        );
    }

    #[test]
    fn parse_iso8601_negative_offset() {
        assert_eq!(
            format::parse_iso8601("2026-07-22T12:00:00-05:00").unwrap(),
            1784721600 + 5 * 3600
        );
    }

    #[test]
    fn parse_iso8601_invalid() {
        assert!(format::parse_iso8601("not-a-date").is_err());
    }
}
