mod drand;
mod format;

use sha2::{Digest, Sha256};
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
}

fn parse_args(args: &[String]) -> Result<Config, String> {
    let mut round: Option<u64> = None;
    let mut output = Output::Human;
    let mut options: Vec<String> = Vec::new();
    let mut file: Option<String> = None;
    let mut delimiter: Option<String> = None;
    let mut quiet = false;
    let mut at: Option<String> = None;

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
            "--quiet" | "-q" => quiet = true,
            "--at" => {
                i += 1;
                at = Some(args.get(i).ok_or("--at requires a timestamp")?.clone());
            }
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
            _ => options.push(args[i].clone()),
        }
        i += 1;
    }

    let input_hash = if let Some(ref path) = file {
        let contents = fs::read(path).map_err(|e| format!("cannot read {path}: {e}"))?;
        let hash = hex_sha256(&contents);
        let text =
            String::from_utf8(contents).map_err(|e| format!("file is not valid UTF-8: {e}"))?;
        let delim = delimiter.as_deref().unwrap_or("\n");
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

    if file.is_some() && matches!(output, Output::Sh | Output::Fish | Output::Ps | Output::All) {
        return Err(
            "--sh/--fish/--ps/--all cannot be used with --file (use --json or --tsv instead)"
                .to_string(),
        );
    }

    if at.is_some() && round.is_some() {
        return Err("--at and --round cannot be used together".to_string());
    }

    if let Some(ref ts) = at {
        let epoch = parse_iso8601(ts)?;
        if epoch <= drand::GENESIS_TIME {
            return Err("--at timestamp is before drand genesis".to_string());
        }
        round = Some((epoch - drand::GENESIS_TIME) / drand::PERIOD);
    }

    Ok(Config {
        round,
        output,
        options,
        input_hash,
        file,
        delimiter,
        quiet,
        at_mode: at.is_some(),
    })
}

/// Derive a selection index from hex randomness and option count.
pub fn select(randomness: &str, count: usize) -> Result<usize, String> {
    if randomness.len() < 8 {
        return Err("randomness too short".to_string());
    }
    let val = u32::from_str_radix(&randomness[..8], 16)
        .map_err(|e| format!("invalid randomness hex: {e}"))?;
    Ok(val as usize % count)
}

pub fn hex_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

/// Parse ISO 8601 timestamp (with timezone offset or Z) to unix epoch seconds.
fn parse_iso8601(s: &str) -> Result<u64, String> {
    // Supports: 2026-07-22T12:00:00Z, 2026-07-22T12:00:00+03:00, 2026-07-22T12:00:00-05:00
    let err =
        || format!("invalid timestamp: {s} (expected ISO 8601, e.g. 2026-07-22T12:00:00+03:00)");

    let (datetime_str, offset_secs) = if let Some(stripped) = s.strip_suffix('Z') {
        (stripped, 0i64)
    } else if s.len() >= 6
        && (s.as_bytes()[s.len() - 6] == b'+' || s.as_bytes()[s.len() - 6] == b'-')
    {
        let (dt, tz) = s.split_at(s.len() - 6);
        let sign: i64 = if tz.starts_with('-') { -1 } else { 1 };
        let h: i64 = tz[1..3].parse().map_err(|_| err())?;
        let m: i64 = tz[4..6].parse().map_err(|_| err())?;
        (dt, sign * (h * 3600 + m * 60))
    } else {
        return Err(err());
    };

    // Parse datetime: 2026-07-22T12:00:00
    let parts: Vec<&str> = datetime_str.split('T').collect();
    if parts.len() != 2 {
        return Err(err());
    }
    let date_parts: Vec<u64> = parts[0]
        .split('-')
        .map(|p| p.parse().unwrap_or(0))
        .collect();
    let time_parts: Vec<u64> = parts[1]
        .split(':')
        .map(|p| p.parse().unwrap_or(0))
        .collect();
    if date_parts.len() != 3 || time_parts.len() != 3 {
        return Err(err());
    }

    let (year, month, day) = (date_parts[0] as i64, date_parts[1], date_parts[2]);
    let (hour, min, sec) = (time_parts[0], time_parts[1], time_parts[2]);

    // Days from epoch to start of year
    let mut days: i64 = 0;
    for y in 1970..year {
        days += if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
            366
        } else {
            365
        };
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let mdays: [i64; 12] = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    for d in &mdays[..month as usize - 1] {
        days += d;
    }
    days += (day as i64) - 1;

    let epoch =
        days * 86400 + (hour as i64) * 3600 + (min as i64) * 60 + (sec as i64) - offset_secs;
    Ok(epoch as u64)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let config = parse_args(&args).unwrap_or_else(|e| {
        if e.is_empty() {
            format::print_usage();
            process::exit(0);
        }
        eprintln!("error: {e}");
        eprintln!();
        format::print_usage();
        process::exit(2);
    });

    if config.at_mode {
        let round = config.round.unwrap();
        let timestamp = format::epoch_to_iso((drand::GENESIS_TIME + round * drand::PERIOD) as i64);
        let verify_args = if let Some(ref file) = config.file {
            let basename = std::path::Path::new(file.as_str())
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_else(|| file.clone());
            let mut s = format!("--file {}", format::shell_quote(&basename));
            if let Some(ref d) = config.delimiter {
                s.push_str(&format!(" --delimiter {}", format::shell_quote(d)));
            }
            s
        } else {
            config
                .options
                .iter()
                .map(|o| format::shell_quote(o))
                .collect::<Vec<_>>()
                .join(" ")
        };

        if config.quiet {
            println!("alea --round {round} {verify_args}");
        } else {
            println!("Scheduled alea run:");
            println!();
            println!("round: {round}");
            println!("time:  {timestamp}");
            if let Some(ref hash) = config.input_hash {
                println!("input: sha256:{hash}");
            }
            println!("count: {} options", config.options.len());
            println!();
            println!("run at the scheduled time:");
            println!("  alea --round {round} {verify_args}");
        }
        return;
    }

    let data = drand::fetch(config.round).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        process::exit(1);
    });

    let index = select(&data.randomness, config.options.len()).unwrap_or_else(|e| {
        eprintln!("error: {e}");
        process::exit(1);
    });

    let result = format::SelectionResult {
        round: data.round,
        randomness: &data.randomness,
        index,
        winner: &config.options[index],
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
    fn parse_args_basic() {
        let args: Vec<String> = vec!["Alice", "Bob"].into_iter().map(String::from).collect();
        let config = parse_args(&args).unwrap();
        assert_eq!(config.options, vec!["Alice", "Bob"]);
        assert!(config.round.is_none());
        assert!(config.input_hash.is_none());
    }

    #[test]
    fn parse_args_with_round() {
        let args: Vec<String> = vec!["--round", "123", "A", "B"]
            .into_iter()
            .map(String::from)
            .collect();
        let config = parse_args(&args).unwrap();
        assert_eq!(config.round, Some(123));
        assert_eq!(config.options, vec!["A", "B"]);
    }

    #[test]
    fn parse_args_too_few_options() {
        let args: Vec<String> = vec!["only_one"].into_iter().map(String::from).collect();
        assert!(parse_args(&args).is_err());
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
        let config = parse_args(&args).unwrap();
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
        let config = parse_args(&args).unwrap();
        assert_eq!(config.options, vec!["Alice", "Bob", "Charlie"]);
        fs::remove_file(tmp).ok();
    }

    #[test]
    fn hex_sha256_known() {
        // SHA-256 of empty string
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
        assert_eq!(parse_iso8601("2026-07-22T12:00:00Z").unwrap(), 1784721600);
    }

    #[test]
    fn parse_iso8601_positive_offset() {
        // +03:00 means local is 3h ahead, so UTC is 3h earlier
        assert_eq!(
            parse_iso8601("2026-07-22T12:00:00+03:00").unwrap(),
            1784721600 - 3 * 3600
        );
    }

    #[test]
    fn parse_iso8601_negative_offset() {
        assert_eq!(
            parse_iso8601("2026-07-22T12:00:00-05:00").unwrap(),
            1784721600 + 5 * 3600
        );
    }

    #[test]
    fn parse_iso8601_invalid() {
        assert!(parse_iso8601("not-a-date").is_err());
    }
}
