use serde::Deserialize;
use std::process;

#[derive(Deserialize)]
struct DrandResponse {
    round: u64,
    randomness: String,
}

const GENESIS_TIME: u64 = 1595431050;
const PERIOD: u64 = 30;

enum Output { Human, Json, Tsv, Sh, Fish, Ps }

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut round: Option<u64> = None;
    let mut output = Output::Human;
    let mut options: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--round" => {
                i += 1;
                round = Some(args.get(i).unwrap_or_else(|| {
                    eprintln!("error: --round requires a value");
                    process::exit(2);
                }).parse().unwrap_or_else(|_| {
                    eprintln!("error: invalid round number");
                    process::exit(2);
                }));
            }
            "--json" => output = Output::Json,
            "--tsv" => output = Output::Tsv,
            "--sh" => output = Output::Sh,
            "--fish" => output = Output::Fish,
            "--ps" => output = Output::Ps,
            "--help" | "-h" => {
                print_usage();
                process::exit(0);
            }
            s if s.starts_with('-') => {
                eprintln!("error: unknown flag: {s}");
                process::exit(2);
            }
            _ => options.push(args[i].clone()),
        }
        i += 1;
    }

    if options.len() < 2 {
        eprintln!("error: at least 2 options required");
        eprintln!();
        print_usage();
        process::exit(2);
    }

    let url = match round {
        Some(r) => format!("https://api.drand.sh/public/{r}"),
        None => "https://api.drand.sh/public/latest".to_string(),
    };

    let body: String = ureq::get(&url).call().unwrap_or_else(|e| {
        eprintln!("error: drand request failed: {e}");
        process::exit(1);
    }).into_body().read_to_string().unwrap_or_else(|e| {
        eprintln!("error: failed to read response: {e}");
        process::exit(1);
    });

    let data: DrandResponse = serde_json::from_str(&body).unwrap_or_else(|e| {
        eprintln!("error: failed to parse drand response: {e}");
        process::exit(1);
    });

    let round = data.round;
    let index = u32::from_str_radix(&data.randomness[..8], 16).unwrap_or_else(|e| {
        eprintln!("error: failed to parse randomness: {e}");
        process::exit(1);
    }) as usize % options.len();
    let winner = &options[index];
    let timestamp = epoch_to_iso((GENESIS_TIME + round * PERIOD) as i64);

    match output {
        Output::Human => {
            println!("🎲 {winner}");
            println!();
            println!("round: {round}");
            println!("time:  {timestamp}");
            println!();
            let quoted: String = options.iter().map(|o| shell_quote(o)).collect::<Vec<_>>().join(" ");
            println!("verify: alea --round {round} {quoted}");
        }
        Output::Json => {
            println!(
                r#"{{"round":{round},"randomness":"{}","index":{index},"winner":"{}","timestamp":"{timestamp}","options":[{}]}}"#,
                data.randomness,
                winner.replace('\\', "\\\\").replace('"', "\\\""),
                options.iter().map(|o| format!("\"{}\"", o.replace('\\', "\\\\").replace('"', "\\\""))).collect::<Vec<_>>().join(",")
            );
        }
        Output::Tsv => {
            println!("round\t{round}");
            println!("randomness\t{}", data.randomness);
            println!("index\t{index}");
            println!("winner\t{winner}");
            println!("timestamp\t{timestamp}");
            println!("options\t{}", options.join("\t"));
        }
        Output::Sh => {
            let quoted: String = options.iter().map(|o| shell_quote(o)).collect::<Vec<_>>().join(" ");
            println!(
                r#"opts=({quoted}); r=$(curl -s https://api.drand.sh/public/{round} | grep -o '"randomness":"[^"]*"' | cut -d'"' -f4); i=$(printf "%d" "0x${{r:0:8}}"); echo "${{opts[$((i % ${{#opts[@]}}))]}}""#
            );
        }
        Output::Fish => {
            let quoted: String = options.iter().map(|o| shell_quote(o)).collect::<Vec<_>>().join(" ");
            println!(
                r#"set opts {quoted}; set r (curl -s https://api.drand.sh/public/{round} | grep -o '"randomness":"[^"]*"' | cut -d'"' -f4); set i (printf "%d" "0x"(string sub -l 8 $r)); math (math $i % (count $opts)) + 1 | read idx; echo $opts[$idx]"#
            );
        }
        Output::Ps => {
            let quoted: String = options.iter().map(|o| ps_quote(o)).collect::<Vec<_>>().join(",");
            println!(
                r#"$opts=@({quoted});$r=(Invoke-RestMethod https://api.drand.sh/public/{round}).randomness;$i=[Convert]::ToUInt32($r.Substring(0,8),16);$opts[$i%$opts.Count]"#
            );
        }
    }
}

fn shell_quote(s: &str) -> String {
    if s.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/') {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

fn ps_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

fn print_usage() {
    eprintln!("Usage: alea [OPTIONS] <option1> <option2> [option3...]");
    eprintln!();
    eprintln!("Verifiable random selection using drand public randomness.");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --round <N>  Use a specific drand round (for verification)");
    eprintln!("  --json       Machine-readable JSON output");
    eprintln!("  --tsv        Tab-separated key/value output (grep/awk/cut friendly)");
    eprintln!("  --sh         Output bash/zsh verification oneliner");
    eprintln!("  --fish       Output fish verification oneliner");
    eprintln!("  --ps         Output PowerShell verification oneliner");
    eprintln!("  -h, --help   Show this help");
}

fn epoch_to_iso(epoch: i64) -> String {
    let days = epoch / 86400;
    let rem = epoch % 86400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let mut y = 1970i64;
    let mut d = days;
    loop {
        let yd = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
        if d < yd { break; }
        d -= yd;
        y += 1;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let mdays = [31, if leap {29} else {28}, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut mo = 0usize;
    while mo < 12 && d >= mdays[mo] { d -= mdays[mo]; mo += 1; }
    format!("{y:04}-{:02}-{:02}T{h:02}:{m:02}:{s:02}Z", mo + 1, d + 1)
}
