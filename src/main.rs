mod drand;
mod format;

use std::process;

use format::Output;

#[derive(Debug)]
struct Config {
    round: Option<u64>,
    output: Output,
    options: Vec<String>,
}

fn parse_args(args: &[String]) -> Result<Config, String> {
    let mut round: Option<u64> = None;
    let mut output = Output::Human;
    let mut options: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--round" => {
                i += 1;
                let val = args.get(i).ok_or("--round requires a value")?;
                round = Some(val.parse().map_err(|_| "invalid round number")?);
            }
            "--json" => output = Output::Json,
            "--tsv" => output = Output::Tsv,
            "--sh" => output = Output::Sh,
            "--fish" => output = Output::Fish,
            "--ps" => output = Output::Ps,
            "--help" | "-h" => return Err(String::new()),
            s if s.starts_with('-') => return Err(format!("unknown flag: {s}")),
            _ => options.push(args[i].clone()),
        }
        i += 1;
    }

    if options.len() < 2 {
        return Err("at least 2 options required".to_string());
    }

    Ok(Config { round, output, options })
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
    };

    format::render(&result, &config.output);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_deterministic() {
        // 0x3b26244e = 992_478_286; 992_478_286 % 3 = 1
        assert_eq!(select("3b26244efd679a692b8bff80fb16b74f", 3).unwrap(), 1);
    }

    #[test]
    fn select_two_options() {
        // 0xffffffff = 4_294_967_295; 4_294_967_295 % 2 = 1
        assert_eq!(select("ffffffff0000000000000000", 2).unwrap(), 1);
        // 0x00000000 = 0; 0 % 2 = 0
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
    }

    #[test]
    fn parse_args_with_round() {
        let args: Vec<String> = vec!["--round", "123", "A", "B"].into_iter().map(String::from).collect();
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
        let args: Vec<String> = vec!["--bogus", "A", "B"].into_iter().map(String::from).collect();
        let err = parse_args(&args).unwrap_err();
        assert!(err.contains("unknown flag"));
    }

    #[test]
    fn parse_args_round_missing_value() {
        let args: Vec<String> = vec!["A", "B", "--round"].into_iter().map(String::from).collect();
        assert!(parse_args(&args).is_err());
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
}
