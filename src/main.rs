use serde::Deserialize;
use std::process;

#[derive(Deserialize)]
struct DrandResponse {
    round: u64,
    randomness: String,
}

const GENESIS_TIME: u64 = 1595431050;
const PERIOD: u64 = 30;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let mut round: Option<u64> = None;
    let mut sh_mode = false;
    let mut options: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--round" => {
                i += 1;
                round = Some(args[i].parse().unwrap_or_else(|_| {
                    eprintln!("Invalid round number");
                    process::exit(1);
                }));
            }
            "--sh" => sh_mode = true,
            _ => options.push(args[i].clone()),
        }
        i += 1;
    }

    if options.len() < 2 {
        eprintln!("Usage: alea [--round N] [--sh] <option1> <option2> [option3...]");
        process::exit(1);
    }

    let url = match round {
        Some(r) => format!("https://api.drand.sh/public/{r}"),
        None => "https://api.drand.sh/public/latest".to_string(),
    };

    let body: String = ureq::get(&url).call().unwrap_or_else(|e| {
        eprintln!("drand request failed: {e}");
        process::exit(1);
    }).into_body().read_to_string().unwrap_or_else(|e| {
        eprintln!("Failed to read response: {e}");
        process::exit(1);
    });

    let data: DrandResponse = serde_json::from_str(&body).unwrap_or_else(|e| {
        eprintln!("Failed to parse drand response: {e}");
        process::exit(1);
    });

    let round = data.round;
    let index = u32::from_str_radix(&data.randomness[..8], 16).unwrap_or_else(|e| {
        eprintln!("Failed to parse randomness: {e}");
        process::exit(1);
    }) as usize % options.len();
    let winner = &options[index];

    let timestamp = (GENESIS_TIME + round * PERIOD) as i64;
    let secs = timestamp;
    let dt = chrono_free_iso(secs);

    if sh_mode {
        let quoted: String = options.iter().map(|o| format!("\"{o}\"")).collect::<Vec<_>>().join(" ");
        println!(
            r#"opts=({quoted}); r=$(curl -s https://api.drand.sh/public/{round} | grep -o '"randomness":"[^"]*"' | cut -d'"' -f4); i=$(printf "%d" "0x${{r:0:8}}"); echo "${{opts[$((i % ${{#opts[@]}}))]}}""#
        );
    } else {
        let quoted: String = options.iter().map(|o| format!("\"{o}\"")).collect::<Vec<_>>().join(" ");
        println!("🎲 Result: {winner}");
        println!("   Time:   {dt}");
        println!();
        println!("Verify:");
        println!("  alea --round {round} {quoted}");
    }
}

fn chrono_free_iso(epoch: i64) -> String {
    let days = epoch / 86400;
    let rem = epoch % 86400;
    let h = rem / 3600;
    let m = (rem % 3600) / 60;
    let s = rem % 60;

    // Days since 1970-01-01 to Y-M-D
    let mut y = 1970i64;
    let mut d = days;
    loop {
        let yd = if is_leap(y) { 366 } else { 365 };
        if d < yd { break; }
        d -= yd;
        y += 1;
    }
    let leap = is_leap(y);
    let mdays = [31, if leap {29} else {28}, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut mo = 0usize;
    while mo < 12 && d >= mdays[mo] {
        d -= mdays[mo];
        mo += 1;
    }
    format!("{y:04}-{:02}-{:02}T{h:02}:{m:02}:{s:02}Z", mo + 1, d + 1)
}

fn is_leap(y: i64) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}
