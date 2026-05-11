use serde::Deserialize;

const DRAND_BASE: &str = "https://api.drand.sh/public";

#[derive(Deserialize)]
pub struct DrandResponse {
    pub round: u64,
    pub randomness: String,
}

pub fn fetch(round: Option<u64>) -> Result<DrandResponse, String> {
    let url = match round {
        Some(r) => format!("{DRAND_BASE}/{r}"),
        None => format!("{DRAND_BASE}/latest"),
    };

    let body = ureq::get(&url)
        .call()
        .map_err(|e| format!("drand request failed: {e}"))?
        .into_body()
        .read_to_string()
        .map_err(|e| format!("failed to read response: {e}"))?;

    serde_json::from_str(&body).map_err(|e| format!("failed to parse drand response: {e}"))
}
