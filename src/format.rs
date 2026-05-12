use serde::Serialize;

use crate::{drand, time};

#[derive(Debug)]
pub enum Output {
    Human,
    All,
    Json,
    Tsv,
    Sh,
    Fish,
    Ps,
}

pub struct SelectionResult<'a> {
    pub round: u64,
    pub randomness: &'a str,
    pub selected: Vec<usize>,
    pub options: &'a [String],
    pub input_hash: Option<&'a str>,
    pub file: Option<&'a str>,
    pub delimiter: Option<&'a str>,
}

impl<'a> SelectionResult<'a> {
    fn count(&self) -> usize {
        self.selected.len()
    }

    fn winner(&self) -> &str {
        &self.options[self.selected[0]]
    }

    fn first_index(&self) -> usize {
        self.selected[0]
    }

    fn winners(&self) -> Vec<&str> {
        self.selected
            .iter()
            .map(|&i| self.options[i].as_str())
            .collect()
    }
}

pub fn render(r: &SelectionResult, output: &Output, quiet: bool) {
    let timestamp = round_to_iso(r.round);

    match output {
        Output::Human => {
            if quiet {
                if r.count() == 1 {
                    println!("{}", r.winner());
                } else {
                    for w in r.winners() {
                        println!("{w}");
                    }
                }
            } else {
                print_header(r, &timestamp);
                if r.file != Some("-") {
                    println!();
                    println!("verify:");
                    println!(
                        "  alea --round {} {}",
                        r.round,
                        verify_args(r.file, r.delimiter, r.options, r.count())
                    );
                }
            }
        }
        Output::All => {
            if !quiet {
                print_header(r, &timestamp);
                println!();
            }
            println!("verify (alea):");
            println!(
                "  alea --round {} {}",
                r.round,
                verify_args(r.file, r.delimiter, r.options, r.count())
            );
            println!();
            println!("verify (bash/zsh):");
            print_indented(&oneliner_sh(r));
            println!();
            println!("verify (fish):");
            print_indented(&oneliner_fish(r));
            println!();
            println!("verify (PowerShell):");
            print_indented(&oneliner_ps(r));
        }
        Output::Json => {
            #[derive(Serialize)]
            struct JsonOut<'a> {
                round: u64,
                randomness: &'a str,
                #[serde(skip_serializing_if = "Option::is_none")]
                index: Option<usize>,
                #[serde(skip_serializing_if = "Option::is_none")]
                winner: Option<&'a str>,
                winners: Vec<&'a str>,
                timestamp: &'a str,
                options: &'a [String],
                #[serde(skip_serializing_if = "Option::is_none")]
                input_hash: Option<&'a str>,
            }
            let winners = r.winners();
            let (index, winner) = if r.count() == 1 {
                (Some(r.first_index()), Some(r.winner()))
            } else {
                (None, None)
            };
            let out = JsonOut {
                round: r.round,
                randomness: r.randomness,
                index,
                winner,
                winners,
                timestamp: &timestamp,
                options: r.options,
                input_hash: r.input_hash,
            };
            println!("{}", serde_json::to_string(&out).unwrap());
        }
        Output::Tsv => {
            println!("round\t{}", r.round);
            println!("randomness\t{}", r.randomness);
            println!("timestamp\t{timestamp}");
            if r.count() == 1 {
                println!("index\t{}", r.first_index());
                println!("winner\t{}", r.winner());
            } else {
                for (rank, w) in r.winners().iter().enumerate() {
                    println!("winner\t{}\t{w}", rank + 1);
                }
            }
            if let Some(hash) = r.input_hash {
                println!("input_hash\t{hash}");
            }
            println!("options\t{}", r.options.join("\t"));
        }
        Output::Sh => {
            if quiet {
                println!("{}", oneliner_sh(r));
            } else {
                print_header(r, &timestamp);
                println!();
                println!("verify (bash/zsh):");
                print_indented(&oneliner_sh(r));
            }
        }
        Output::Fish => {
            if quiet {
                println!("{}", oneliner_fish(r));
            } else {
                print_header(r, &timestamp);
                println!();
                println!("verify (fish):");
                print_indented(&oneliner_fish(r));
            }
        }
        Output::Ps => {
            if quiet {
                println!("{}", oneliner_ps(r));
            } else {
                print_header(r, &timestamp);
                println!();
                println!("verify (PowerShell):");
                print_indented(&oneliner_ps(r));
            }
        }
    }
}

pub struct ScheduledResult<'a> {
    pub round: u64,
    pub options: &'a [String],
    pub input_hash: Option<&'a str>,
    pub file: Option<&'a str>,
    pub delimiter: Option<&'a str>,
    pub count: usize,
    pub past: bool,
}

pub fn render_scheduled(r: &ScheduledResult, quiet: bool) {
    let round = r.round;
    let timestamp = round_to_iso(round);
    let args = verify_args(r.file, r.delimiter, r.options, r.count);

    if quiet {
        println!("alea --round {round} {args}");
    } else {
        if r.past {
            println!("Historical alea run:");
        } else {
            println!("Scheduled alea run:");
        }
        println!();
        println!("round: {round}");
        println!("time:  {timestamp}");
        if let Some(hash) = r.input_hash {
            println!("input: sha256:{hash}");
        }
        println!("count: {} options", r.options.len());
        if r.count > 1 {
            println!("picks: {}", r.count);
        }
        println!();
        if r.past {
            println!("run now:");
        } else {
            println!("run at the scheduled time:");
        }
        println!("  alea --round {round} {args}");
    }
}

fn round_to_iso(round: u64) -> String {
    time::epoch_to_iso((drand::GENESIS_TIME + round * drand::PERIOD) as i64)
}

fn print_indented(s: &str) {
    for line in s.lines() {
        println!("  {line}");
    }
}

fn print_header(r: &SelectionResult, timestamp: &str) {
    if r.count() == 1 {
        println!("\u{1f3b2} {}", r.winner());
    } else {
        for (rank, w) in r.winners().iter().enumerate() {
            if rank == 0 {
                println!("\u{1f3b2} {}. {w}", rank + 1);
            } else {
                println!("   {}. {w}", rank + 1);
            }
        }
    }
    println!();
    println!("round: {}", r.round);
    println!("time:  {timestamp}");
    if let Some(hash) = r.input_hash {
        println!("input: sha256:{hash}");
    }
}

fn verify_args(
    file: Option<&str>,
    delimiter: Option<&str>,
    options: &[String],
    count: usize,
) -> String {
    let count_prefix = if count > 1 {
        format!("--count {count} ")
    } else {
        String::new()
    };

    if let Some(f) = file {
        let basename = std::path::Path::new(f)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| f.to_string());
        let mut s = format!("{count_prefix}--file {}", shell_quote(&basename));
        if let Some(d) = delimiter {
            s.push_str(&format!(" --delimiter {}", shell_quote(d)));
        }
        s
    } else {
        format!("{count_prefix}{}", quote_all(options, shell_quote))
    }
}

fn sanitize_comment(s: &str) -> String {
    s.replace('\r', "\\r").replace('\n', "\\n")
}

fn oneliner_comment(r: &SelectionResult) -> String {
    format!(
        "# alea {} --round {} => {}",
        sanitize_comment(&verify_args(r.file, r.delimiter, r.options, r.count())),
        r.round,
        sanitize_comment(r.winner())
    )
}

fn oneliner_sh(r: &SelectionResult) -> String {
    let quoted = quote_all(r.options, shell_quote);
    format!(
        "{}\nopts=({quoted}); r=$(curl -s https://api.drand.sh/public/{} | grep -o '\"randomness\":\"[^\"]*\"' | cut -d'\"' -f4); i=$(printf \"%d\" \"0x${{r:0:8}}\"); echo \"${{opts[@]:$((i % ${{#opts[@]}})):1}}\"",
        oneliner_comment(r),
        r.round
    )
}

fn oneliner_fish(r: &SelectionResult) -> String {
    let quoted = quote_all(r.options, shell_quote);
    format!(
        "{}\nset opts {quoted}; set r (curl -s https://api.drand.sh/public/{} | grep -o '\"randomness\":\"[^\"]*\"' | cut -d'\"' -f4); set i (printf \"%d\" \"0x\"(string sub -l 8 $r)); math (math $i % (count $opts)) + 1 | read idx; echo $opts[$idx]",
        oneliner_comment(r),
        r.round
    )
}

fn oneliner_ps(r: &SelectionResult) -> String {
    let quoted = r
        .options
        .iter()
        .map(|o| ps_quote(o))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{}\n$opts=@({quoted});$r=(Invoke-RestMethod https://api.drand.sh/public/{}).randomness;$i=[Convert]::ToUInt32($r.Substring(0,8),16);$opts[$i%$opts.Count]",
        oneliner_comment(r),
        r.round
    )
}

pub fn print_usage() {
    eprintln!(
        "alea {} - verifiable random selection using drand",
        env!("CARGO_PKG_VERSION")
    );
    eprintln!();
    eprintln!("Usage: alea [OPTIONS] <option1> <option2> [option3...]");
    eprintln!("       alea [OPTIONS] --file <path> [--delimiter <delim>]");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --round <N>       Use a specific drand round (for verification)");
    eprintln!("  --at <TIMESTAMP>  Calculate round for a future time (ISO 8601)");
    eprintln!("  -f, --file <path> Read options from a file");
    eprintln!("  -d, --delimiter <str> Split file by delimiter (default: newline)");
    eprintln!("  -n, --count <N>   Pick N unique winners (default: 1, max: 8)");
    eprintln!("  -q, --quiet       Print only the result, no headers or labels");
    eprintln!("  --all             Show all verification methods");
    eprintln!("  --json            Machine-readable JSON output");
    eprintln!("  --tsv             Tab-separated key/value output (grep/awk/cut friendly)");
    eprintln!("  --sh              Output bash/zsh verification oneliner (single winner only)");
    eprintln!("  --fish            Output fish verification oneliner (single winner only)");
    eprintln!("  --ps              Output PowerShell verification oneliner (single winner only)");
    eprintln!("  -V, --version     Show version");
    eprintln!("  -h, --help        Show this help");
}

fn quote_all(options: &[String], quoter: fn(&str) -> String) -> String {
    options
        .iter()
        .map(|o| quoter(o))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn shell_quote(s: &str) -> String {
    if s.chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/')
    {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

fn ps_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}
