use serde::Serialize;

use crate::drand;

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
    pub index: usize,
    pub winner: &'a str,
    pub options: &'a [String],
    pub input_hash: Option<&'a str>,
    pub file: Option<&'a str>,
    pub delimiter: Option<&'a str>,
}

pub fn render(r: &SelectionResult, output: &Output, quiet: bool) {
    let timestamp = epoch_to_iso((drand::GENESIS_TIME + r.round * drand::PERIOD) as i64);

    match output {
        Output::Human => {
            if quiet {
                println!("{}", r.winner);
            } else {
                print_header(r, &timestamp);
                println!();
                println!("verify:");
                println!("  alea --round {} {}", r.round, verify_args(r));
            }
        }
        Output::All => {
            if !quiet {
                print_header(r, &timestamp);
                println!();
            }
            println!("verify (alea):");
            println!("  alea --round {} {}", r.round, verify_args(r));
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
                index: usize,
                winner: &'a str,
                timestamp: &'a str,
                options: &'a [String],
                #[serde(skip_serializing_if = "Option::is_none")]
                input_hash: Option<&'a str>,
            }
            let out = JsonOut {
                round: r.round,
                randomness: r.randomness,
                index: r.index,
                winner: r.winner,
                timestamp: &timestamp,
                options: r.options,
                input_hash: r.input_hash,
            };
            println!("{}", serde_json::to_string(&out).unwrap());
        }
        Output::Tsv => {
            println!("round\t{}", r.round);
            println!("randomness\t{}", r.randomness);
            println!("index\t{}", r.index);
            println!("winner\t{}", r.winner);
            println!("timestamp\t{timestamp}");
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

fn print_indented(s: &str) {
    for line in s.lines() {
        println!("  {line}");
    }
}

fn print_header(r: &SelectionResult, timestamp: &str) {
    println!("🎲 {}", r.winner);
    println!();
    println!("round: {}", r.round);
    println!("time:  {timestamp}");
    if let Some(hash) = r.input_hash {
        println!("input: sha256:{hash}");
    }
}

fn verify_args(r: &SelectionResult) -> String {
    if let Some(file) = r.file {
        let basename = std::path::Path::new(file)
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_else(|| file.to_string());
        let mut s = format!("--file {}", shell_quote(&basename));
        if let Some(d) = r.delimiter {
            s.push_str(&format!(" --delimiter {}", shell_quote(d)));
        }
        s
    } else {
        quote_all(r.options, shell_quote)
    }
}

fn sanitize_comment(s: &str) -> String {
    s.replace('\r', "\\r").replace('\n', "\\n")
}

fn oneliner_comment(r: &SelectionResult) -> String {
    format!(
        "# alea {} --round {} => {}",
        sanitize_comment(&verify_args(r)),
        r.round,
        sanitize_comment(r.winner)
    )
}

fn oneliner_sh(r: &SelectionResult) -> String {
    let quoted = quote_all(r.options, shell_quote);
    format!(
        "{}\nopts=({quoted}); r=$(curl -s https://api.drand.sh/public/{} | grep -o '\"randomness\":\"[^\"]*\"' | cut -d'\"' -f4); i=$(printf \"%d\" \"0x${{r:0:8}}\"); echo \"${{opts[$((i % ${{#opts[@]}}))]}}\"",
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
    eprintln!("  -f, --file <path> Read options from a file");
    eprintln!("  -d, --delimiter <str> Split file by delimiter (default: newline)");
    eprintln!(
        "  -q, --quiet       Print only the result, no headers or labels
  --all             Show all verification methods"
    );
    eprintln!("  --json            Machine-readable JSON output");
    eprintln!("  --tsv             Tab-separated key/value output (grep/awk/cut friendly)");
    eprintln!("  --sh              Output bash/zsh verification oneliner");
    eprintln!("  --fish            Output fish verification oneliner");
    eprintln!("  --ps              Output PowerShell verification oneliner");
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

pub fn epoch_to_iso(epoch: i64) -> String {
    let days = epoch / 86400;
    let rem = epoch % 86400;
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let mut y = 1970i64;
    let mut d = days;
    loop {
        let yd = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
            366
        } else {
            365
        };
        if d < yd {
            break;
        }
        d -= yd;
        y += 1;
    }
    let leap = y % 4 == 0 && (y % 100 != 0 || y % 400 == 0);
    let mdays = [
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
    let mut mo = 0usize;
    while mo < 12 && d >= mdays[mo] {
        d -= mdays[mo];
        mo += 1;
    }
    format!("{y:04}-{:02}-{:02}T{h:02}:{m:02}:{s:02}Z", mo + 1, d + 1)
}
