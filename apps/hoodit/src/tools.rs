use crate::{
    app::{HooditApp, ReadContext},
    model,
    providers::*,
};
use aomi_sdk::schemars::JsonSchema;
use aomi_sdk::{DynAomiTool, DynToolCallCtx};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashSet;
macro_rules! invalid {
    ($value:expr) => {
        match $value {
            Ok(value) => value,
            Err(message) => return Ok(model::error("INVALID_ARGUMENT", &message, false)),
        }
    };
}

mod portfolio;
pub use portfolio::*;

fn provider_error(e: ProviderError) -> Value {
    let code = match e.code {
        "INVALID_CURSOR" => "INVALID_ARGUMENT",
        "NO_INDEXED_POOL" | "UPSTREAM_NOT_FOUND" => "POOL_NOT_FOUND",
        "NOT_INDEXED" => "TOKEN_NOT_INDEXED",
        "QUOTE_BUDGET_EXHAUSTED" | "PROVIDER_BUDGET_EXHAUSTED" => "RATE_LIMITED",
        "DEADLINE_EXCEEDED" => "UPSTREAM_UNAVAILABLE",
        other => other,
    };
    model::error(code, &e.message, e.retryable)
}
fn page(page: Option<u8>) -> Result<u8, String> {
    let p = page.unwrap_or(1);
    if !(1..=10).contains(&p) {
        Err("page must be between 1 and 10".into())
    } else {
        Ok(p)
    }
}
fn pool_id(value: &str) -> Result<String, String> {
    let v = value.trim();
    if v.is_empty()
        || v.len() > 200
        || !v
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b':' | b'_' | b'-'))
    {
        Err("invalid opaque pool identifier".into())
    } else {
        Ok(
            if v.starts_with("0x") && v[2..].bytes().all(|b| b.is_ascii_hexdigit()) {
                v.to_ascii_lowercase()
            } else {
                v.to_string()
            },
        )
    }
}
fn data_array(v: &Value) -> Vec<Value> {
    v.get("data")
        .and_then(Value::as_array)
        .cloned()
        .or_else(|| {
            v.get("data")
                .filter(|x| !x.is_null())
                .map(|x| vec![x.clone()])
        })
        .unwrap_or_default()
}
fn selected_pool(
    gecko: &Gecko,
    token: &str,
    explicit: Option<&str>,
    read: &mut ReadContext,
) -> Result<(Value, String), ProviderError> {
    let v = if let Some(id) = explicit {
        gecko.pool(id, read)?
    } else {
        let detail = gecko.token(token, read)?;
        let top = model::get(&detail, &["data", "relationships", "top_pools", "data"])
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(|v| model::string(v, &["id"]))
            .map(|id| id.strip_prefix("robinhood_").unwrap_or(&id).to_string());
        if let Some(id) = top {
            gecko.pool(&id, read)?
        } else {
            gecko.token_pools(token, read)?
        }
    };
    let included = included_map(&v);
    let row = data_array(&v).into_iter().next().ok_or(ProviderError {
        code: "NO_INDEXED_POOL",
        message: "no indexed pool was found".into(),
        retryable: false,
    })?;
    let normalized = pool(&row, &included);
    let id = model::string(&normalized, &["pool_id"]).ok_or(ProviderError {
        code: "UPSTREAM_SCHEMA_CHANGED",
        message: "pool identifier is missing".into(),
        retryable: false,
    })?;
    let contains = ["base_token", "quote_token"]
        .iter()
        .filter_map(|k| model::string(&normalized, &[k, "id"]))
        .any(|id| id.eq_ignore_ascii_case(token));
    if !contains {
        return Err(ProviderError {
            code: "TOKEN_NOT_IN_POOL",
            message: "the token is not a component of the selected pool".into(),
            retryable: false,
        });
    }
    Ok((normalized, id))
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchArgs {
    /// Token name, ticker symbol, or exact 0x contract address to search for.
    /// A name or symbol can return multiple candidates and must not be treated
    /// as an exact token identity.
    pub query: String,
    /// One-based GeckoTerminal search-results page. Omit for page 1; use the
    /// returned next_page value for another page.
    #[serde(default)]
    #[schemars(with = "u8", range(min = 1, max = 10), extend("default" = 1))]
    pub page: Option<u8>,
}
pub struct SearchTokens;
impl DynAomiTool for SearchTokens {
    type App = HooditApp;
    type Args = SearchArgs;
    const NAME: &'static str = "hoodit_search_tokens";
    const DESCRIPTION: &'static str = "Search Robinhood Chain tokens by name, symbol, or exact 0x contract address. Use this before exact-token tools when the user supplied only a name or symbol; return candidates and never guess among ambiguous matches.";
    fn run(app: &HooditApp, args: SearchArgs, _: DynToolCallCtx) -> Result<Value, String> {
        let query = args.query.trim();
        if query.is_empty() || query.len() > 100 {
            return Ok(model::error(
                "INVALID_ARGUMENT",
                "query must contain 1 to 100 characters",
                false,
            ));
        }
        let p = invalid!(page(args.page));
        let runtime = app.runtime()?;
        let mut read = ReadContext::markets(false);
        if let Ok(exact) = model::address(query) {
            let hit=match Gecko::new(&runtime).token(&exact,&mut read) {
                Ok(value)=>data_array(&value).into_iter().next().map(|row|json!({"token":token_from_resource(&row),"match":"address","reference_pool":null,"reference_price_usd":model::string(&row,&["attributes","price_usd"]),"reference_pool_liquidity_usd":null})),
                Err(e) if matches!(e.code,"NOT_INDEXED"|"TOKEN_NOT_INDEXED")=>None,
                Err(e)=>return Ok(provider_error(e)),
            };
            let mut warnings = read.warnings;
            if hit.is_none() {
                warnings.push(model::warning(
                    "NOT_INDEXED",
                    "The exact contract is not indexed by GeckoTerminal",
                ));
            }
            let tokens = hit.into_iter().collect::<Vec<_>>();
            let returned = tokens.len();
            return Ok(model::ok(
                json!({"query":query,"tokens":tokens,"pagination":{"page":1,"page_size":20,"returned":returned,"next_page":null}}),
                read.sources,
                warnings,
            ));
        }
        let value = match Gecko::new(&runtime).search(query, p, &mut read) {
            Ok(v) => v,
            Err(e) => return Ok(provider_error(e)),
        };
        let included = included_map(&value);
        let mut seen = HashSet::new();
        let mut hits = vec![];
        for row in data_array(&value) {
            let pr = pool(&row, &included);
            for side in ["base_token", "quote_token"] {
                if let Some(t) = pr.get(side).filter(|v| !v.is_null()) {
                    let id = model::string(t, &["id"]).unwrap_or_default();
                    if seen.insert(id.clone()) {
                        let symbol = model::string(t, &["symbol"]).unwrap_or_default();
                        let name = model::string(t, &["name"]).unwrap_or_default();
                        let kind = if id.eq_ignore_ascii_case(query) {
                            "address"
                        } else if symbol.eq_ignore_ascii_case(query) {
                            "symbol"
                        } else if name.eq_ignore_ascii_case(query) {
                            "name"
                        } else {
                            "pair"
                        };
                        hits.push(json!({"token":t,"match":kind,"reference_pool":{"pool_id":pr["pool_id"],"dex_id":pr["dex_id"],"dex_name":pr["dex_name"]},"reference_price_usd":if side=="base_token"{pr["base_price_usd"].clone()}else{pr["quote_price_usd"].clone()},"reference_pool_liquidity_usd":pr["liquidity_usd"]}));
                    }
                }
            }
        }
        hits.sort_by_key(|hit| match model::string(hit, &["match"]).as_deref() {
            Some("address") => 0,
            Some("symbol") => 1,
            Some("name") => 2,
            _ => 3,
        });
        hits.truncate(40);
        let returned = hits.len();
        let raw_full = data_array(&value).len() >= 20;
        let next = if p < 10 && raw_full {
            Some(p + 1)
        } else {
            None
        };
        let mut warnings = read.warnings;
        if p == 10 && raw_full {
            warnings.push(model::warning(
                "PARTIAL_PAGE",
                "The search reached its page limit while more results may exist",
            ));
        }
        Ok(model::ok(
            json!({"query":query,"tokens":hits,"pagination":{"page":p,"page_size":20,"returned":returned,"next_page":next}}),
            read.sources,
            warnings,
        ))
    }
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscoverArgs {
    /// Pool ranking to browse. Use trending unless the user explicitly asks
    /// for newly indexed, highest-volume, or most-active pools.
    #[serde(default)]
    #[schemars(with = "String", extend("enum" = ["trending", "new", "top_volume", "top_activity"], "default" = "trending"))]
    pub feed: Option<String>,
    /// Ranking window for the trending feed. Omit for 24h. Other feeds may not
    /// use this value, and the response states the applied duration.
    #[serde(default)]
    #[schemars(with = "String", extend("enum" = ["5m", "1h", "6h", "24h"], "default" = "24h"))]
    pub duration: Option<String>,
    /// One-based provider page. Omit for page 1; use returned next_page for
    /// another page.
    #[serde(default)]
    #[schemars(with = "u8", range(min = 1, max = 10), extend("default" = 1))]
    pub page: Option<u8>,
    /// Minimum pool liquidity in USD as a non-negative decimal string, for
    /// example "10000". This filters observations; it is not a trade limit.
    #[serde(default)]
    #[schemars(with = "String", pattern(r"^(0|[1-9][0-9]*)(\.[0-9]+)?$"), extend("default" = "0"))]
    pub min_liquidity_usd: Option<String>,
    /// Minimum trailing-24-hour pool volume in USD as a non-negative decimal
    /// string, for example "50000".
    #[serde(default)]
    #[schemars(with = "String", pattern(r"^(0|[1-9][0-9]*)(\.[0-9]+)?$"), extend("default" = "0"))]
    pub min_volume_24h_usd: Option<String>,
}
pub struct DiscoverPools;
impl DynAomiTool for DiscoverPools {
    type App = HooditApp;
    type Args = DiscoverArgs;
    const NAME: &'static str = "hoodit_discover_pools";
    const DESCRIPTION: &'static str = "Browse GeckoTerminal-indexed Robinhood Chain pools by trending, newly indexed, volume, or activity ranking. Use for discovery, not for resolving one named token or obtaining an executable quote.";
    fn run(app: &HooditApp, args: DiscoverArgs, _: DynToolCallCtx) -> Result<Value, String> {
        let feed = args.feed.as_deref().unwrap_or("trending");
        if !["trending", "new", "top_volume", "top_activity"].contains(&feed) {
            return Ok(model::error("INVALID_ARGUMENT", "unsupported feed", false));
        }
        let duration = args.duration.as_deref().unwrap_or("24h");
        if !["5m", "1h", "6h", "24h"].contains(&duration) {
            return Ok(model::error(
                "INVALID_ARGUMENT",
                "unsupported duration",
                false,
            ));
        }
        let p = invalid!(page(args.page));
        let min_l = invalid!(model::decimal(
            args.min_liquidity_usd.as_deref().unwrap_or("0")
        ));
        let min_v = invalid!(model::decimal(
            args.min_volume_24h_usd.as_deref().unwrap_or("0")
        ));
        let runtime = app.runtime()?;
        let mut read = ReadContext::markets(false);
        let value = match Gecko::new(&runtime).discover(feed, duration, p, &mut read) {
            Ok(v) => v,
            Err(e) => return Ok(provider_error(e)),
        };
        let included = included_map(&value);
        let raw_rows = data_array(&value);
        let raw_full = raw_rows.len() >= 20;
        let pools = raw_rows
            .iter()
            .map(|v| pool(v, &included))
            .filter(|v| {
                decimal_ge(model::string(v, &["liquidity_usd"]).as_deref(), &min_l)
                    && decimal_ge(
                        model::string(v, &["windows", "h24", "volume_usd"]).as_deref(),
                        &min_v,
                    )
            })
            .collect::<Vec<_>>();
        let returned = pools.len();
        let next = if p < 10 && raw_full {
            Some(p + 1)
        } else {
            None
        };
        let mut warnings = read.warnings;
        if p == 10 && raw_full {
            warnings.push(model::warning(
                "PARTIAL_PAGE",
                "The feed reached its page limit while more pools may exist",
            ));
        }
        Ok(model::ok(
            json!({"feed":feed,"duration":if feed=="trending"{Some(duration)}else{None},"min_liquidity_usd":min_l,"min_volume_24h_usd":min_v,"pools":pools,"pagination":{"page":p,"page_size":20,"returned":returned,"next_page":next}}),
            read.sources,
            warnings,
        ))
    }
}
fn decimal_ge(actual: Option<&str>, min: &str) -> bool {
    use std::str::FromStr;
    let min = bigdecimal::BigDecimal::from_str(min).ok();
    let actual = actual.and_then(|v| bigdecimal::BigDecimal::from_str(v).ok());
    matches!((actual,min),(Some(a),Some(m)) if a>=m)
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TokenArgs {
    /// Exact Robinhood Chain ERC-20 0x contract address. Do not pass a symbol
    /// or name; resolve those with hoodit_search_tokens first.
    pub token: String,
    /// Optional opaque pool_id returned by a Hoodit search, discovery, token,
    /// candle, or trade result. Omit to use the token's indexed top pool.
    #[serde(default)]
    #[schemars(
        with = "String",
        length(min = 1, max = 200),
        pattern(r"^[A-Za-z0-9:_-]+$")
    )]
    pub pool_id: Option<String>,
    /// Include bounded public project metadata such as description and links.
    /// Omit for false when only market statistics are needed.
    #[serde(default)]
    #[schemars(with = "bool", extend("default" = false))]
    pub include_metadata: Option<bool>,
}
pub struct GetToken;
impl DynAomiTool for GetToken {
    type App = HooditApp;
    type Args = TokenArgs;
    const NAME: &'static str = "hoodit_get_token";
    const DESCRIPTION: &'static str = "Read market statistics and selected-pool context for one exact Robinhood Chain ERC-20 contract. Requires a 0x contract address, not a symbol; use hoodit_search_tokens first when identity is ambiguous. This is observational data, not an executable quote.";
    fn run(app: &HooditApp, args: TokenArgs, _: DynToolCallCtx) -> Result<Value, String> {
        let token = invalid!(model::address(&args.token));
        let explicit = invalid!(args.pool_id.as_deref().map(pool_id).transpose());
        let runtime = app.runtime()?;
        let gecko = Gecko::new(&runtime);
        let mut read = ReadContext::markets(false);
        let detail = match gecko.token(&token, &mut read) {
            Ok(v) => v,
            Err(e) => return Ok(provider_error(e)),
        };
        let Some(row) = data_array(&detail).into_iter().next() else {
            return Ok(model::error(
                "UPSTREAM_SCHEMA_CHANGED",
                "token response is empty",
                false,
            ));
        };
        let attrs = row.get("attributes").cloned().unwrap_or(json!({}));
        let tok = token_from_resource(&row);
        let mut warnings = vec![];
        let top_ids = model::get(&row, &["relationships", "top_pools", "data"])
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|v| model::string(v, &["id"]))
            .map(|id| id.strip_prefix("robinhood_").unwrap_or(&id).to_string())
            .collect::<Vec<_>>();
        let automatic = top_ids.first().map(String::as_str);
        let selected =
            match selected_pool(&gecko, &token, explicit.as_deref().or(automatic), &mut read) {
                Ok((v, _)) => Some(v),
                Err(e) if explicit.is_none() => {
                    warnings.push(model::warning(
                        if e.code == "NO_INDEXED_POOL" {
                            "NOT_INDEXED"
                        } else {
                            "METADATA_UNAVAILABLE"
                        },
                        if e.code == "NO_INDEXED_POOL" {
                            "No indexed reference pool was found"
                        } else {
                            "Token detail is available, but selected-pool context could not be read"
                        },
                    ));
                    None
                }
                Err(e) => return Ok(provider_error(e)),
            };
        let metadata = if args.include_metadata.unwrap_or(false) {
            match gecko.metadata(&token, &mut read) {
                Ok(v) => data_array(&v)
                    .into_iter()
                    .next()
                    .map(|v| normalize_metadata(&v)),
                Err(_) => {
                    warnings.push(model::warning(
                        "METADATA_UNAVAILABLE",
                        "Requested token metadata could not be read",
                    ));
                    None
                }
            }
        } else {
            None
        };
        let selected_price = selected.as_ref().and_then(|p| {
            if model::string(p, &["base_token", "id"]).as_deref() == Some(token.as_str()) {
                p.get("base_price_usd").cloned()
            } else {
                p.get("quote_price_usd").cloned()
            }
        });
        let selected_id = selected
            .as_ref()
            .and_then(|p| model::string(p, &["pool_id"]));
        let mut other_pools = Vec::new();
        for id in top_ids
            .iter()
            .filter(|id| Some(id.as_str()) != selected_id.as_deref())
            .take(4)
        {
            if let Ok(value) = gecko.pool(id, &mut read) {
                let included = included_map(&value);
                if let Some(row) = data_array(&value).into_iter().next() {
                    let p = pool(&row, &included);
                    other_pools.push(json!({"pool_id":p["pool_id"],"dex_id":p["dex_id"],"dex_name":p["dex_name"]}));
                }
            }
        }
        Ok(model::ok(
            json!({"token":tok,"price_usd":model::string(&attrs,&["price_usd"]),"market_cap_usd":model::string(&attrs,&["market_cap_usd"]),"fdv_usd":model::string(&attrs,&["fdv_usd"]),"volume_24h_usd":model::string(&attrs,&["volume_usd","h24"]),"selected_pool":selected,"selected_token_price_usd":selected_price,"other_pools":other_pools,"metadata":metadata}),
            read.sources,
            {
                warnings.extend(read.warnings);
                warnings
            },
        ))
    }
}
fn normalize_metadata(v: &Value) -> Value {
    let a = v.get("attributes").unwrap_or(v);
    let description =
        model::string(a, &["description"]).map(|s| s.chars().take(1000).collect::<String>());
    let websites = a
        .get("websites")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .take(5)
        .collect::<Vec<_>>();
    json!({"description":description,"websites":websites,"twitter":model::string(a,&["twitter_handle"]).or_else(||model::string(a,&["twitter_url"])),"telegram":model::string(a,&["telegram_handle"]).or_else(||model::string(a,&["telegram_url"])),"discord":model::string(a,&["discord_url"])})
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CandlesArgs {
    /// Exact Robinhood Chain ERC-20 0x contract address. Do not pass a symbol
    /// or name; resolve those with hoodit_search_tokens first.
    pub token: String,
    /// Optional opaque pool_id returned by a Hoodit result. Omit to use the
    /// token's indexed top pool. Candles cover only the selected pool.
    #[serde(default)]
    #[schemars(
        with = "String",
        length(min = 1, max = 200),
        pattern(r"^[A-Za-z0-9:_-]+$")
    )]
    pub pool_id: Option<String>,
    /// Candle width. Omit for 1h.
    #[serde(default)]
    #[schemars(with = "String", extend("enum" = ["1m", "5m", "15m", "1h", "4h", "12h", "1d"], "default" = "1h"))]
    pub interval: Option<String>,
    /// Exclusive historical cutoff as a Unix timestamp in whole UTC seconds.
    /// Omit to use the current time. Never pass milliseconds or a future time.
    #[serde(default)]
    #[schemars(with = "i64")]
    pub before: Option<i64>,
    /// Maximum provider candles before open-candle filtering. Omit for 100.
    #[serde(default)]
    #[schemars(with = "u16", range(min = 1, max = 1000), extend("default" = 100))]
    pub limit: Option<u16>,
    /// Include the current incomplete candle. Omit for false when only closed
    /// candles should be compared.
    #[serde(default)]
    #[schemars(with = "bool", extend("default" = false))]
    pub include_open: Option<bool>,
}
pub struct GetCandles;
impl DynAomiTool for GetCandles {
    type App = HooditApp;
    type Args = CandlesArgs;
    const NAME: &'static str = "hoodit_get_candles";
    const DESCRIPTION: &'static str = "Read USD OHLCV history for one exact token contract in one selected pool. Requires a 0x contract address; timestamps are Unix seconds, and results are single-pool market history rather than wallet performance or an executable quote.";
    fn run(app: &HooditApp, args: CandlesArgs, _: DynToolCallCtx) -> Result<Value, String> {
        let token = invalid!(model::address(&args.token));
        let interval = args.interval.as_deref().unwrap_or("1h");
        let (timeframe, aggregate, width) = match interval {
            "1m" => ("minute", 1, 60),
            "5m" => ("minute", 5, 300),
            "15m" => ("minute", 15, 900),
            "1h" => ("hour", 1, 3600),
            "4h" => ("hour", 4, 14400),
            "12h" => ("hour", 12, 43200),
            "1d" => ("day", 1, 86400),
            _ => {
                return Ok(model::error(
                    "INVALID_ARGUMENT",
                    "unsupported candle interval",
                    false,
                ));
            }
        };
        let before = args.before.unwrap_or_else(|| Utc::now().timestamp());
        if before > Utc::now().timestamp() + 5 {
            return Ok(model::error(
                "INVALID_ARGUMENT",
                "before cannot be in the future",
                false,
            ));
        }
        let limit = args.limit.unwrap_or(100);
        if !(1..=1000).contains(&limit) {
            return Ok(model::error(
                "INVALID_ARGUMENT",
                "limit must be 1 to 1000",
                false,
            ));
        }
        let explicit = invalid!(args.pool_id.as_deref().map(pool_id).transpose());
        let runtime = app.runtime()?;
        let gecko = Gecko::new(&runtime);
        let mut read = ReadContext::markets(false);
        let (pool, id) = match selected_pool(&gecko, &token, explicit.as_deref(), &mut read) {
            Ok(v) => v,
            Err(e) => return Ok(provider_error(e)),
        };
        let raw = match gecko.candles(&id, &token, timeframe, aggregate, before, limit, &mut read) {
            Ok(v) => v,
            Err(e) => return Ok(provider_error(e)),
        };
        let rows = model::get(&raw, &["data", "attributes", "ohlcv_list"])
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let raw_full = rows.len() == limit as usize;
        let earliest_raw = rows
            .iter()
            .filter_map(|v| v.as_array())
            .filter_map(|a| a.first())
            .filter_map(Value::as_i64)
            .min();
        let now = Utc::now().timestamp();
        let mut candles=rows.into_iter().filter_map(|v|{let a=v.as_array()?;let ts=a.first()?.as_i64()?;if ts>=before||(!args.include_open.unwrap_or(false)&&ts+width>now){return None}Some(json!({"timestamp":ts,"open":lex(a.get(1)),"high":lex(a.get(2)),"low":lex(a.get(3)),"close":lex(a.get(4)),"volume_usd":lex(a.get(5)),"closed":ts+width<=now}))}).collect::<Vec<_>>();
        candles.sort_by_key(|v| v["timestamp"].as_i64());
        candles.dedup_by_key(|v| v["timestamp"].as_i64());
        let returned = candles.len();
        let has_gaps = candles.windows(2).any(|w| {
            w[1]["timestamp"]
                .as_i64()
                .zip(w[0]["timestamp"].as_i64())
                .is_some_and(|(b, a)| b - a > width)
        });
        let summary = candle_summary(&candles);
        let next = earliest_raw.filter(|_| raw_full);
        let includes_open = candles.iter().any(|v| v["closed"] == false);
        let mut warnings = read.warnings;
        if has_gaps {
            warnings.push(model::warning(
                "GAPS_IN_CANDLES",
                "The returned single-pool candle series contains gaps",
            ));
        }
        if next.is_some() {
            warnings.push(model::warning(
                "HISTORY_LIMITED",
                "Additional older single-pool history may be available",
            ));
        }
        Ok(model::ok(
            json!({"token":token,"pool":{"pool_id":pool["pool_id"],"dex_id":pool["dex_id"],"dex_name":pool["dex_name"]},"interval":interval,"before":before,"currency":"USD","candles":candles,"summary":summary,"coverage":{"scope":"single_pool","requested_limit":limit,"returned":returned,"has_gaps":has_gaps,"includes_open_candle":includes_open,"next_before":next}}),
            read.sources,
            warnings,
        ))
    }
}
fn lex(v: Option<&Value>) -> Option<String> {
    v.and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    })
}
fn candle_summary(rows: &[Value]) -> Option<Value> {
    use std::str::FromStr;
    let first = rows.first()?;
    let last = rows.last()?;
    let nums = |key: &str| {
        rows.iter()
            .filter_map(|v| model::string(v, &[key]))
            .filter_map(|s| bigdecimal::BigDecimal::from_str(&s).ok())
            .collect::<Vec<_>>()
    };
    let highs = nums("high");
    let lows = nums("low");
    let volumes = nums("volume_usd");
    let open = bigdecimal::BigDecimal::from_str(&model::string(first, &["open"])?).ok()?;
    let close = bigdecimal::BigDecimal::from_str(&model::string(last, &["close"])?).ok()?;
    let change = if num_traits::Zero::is_zero(&open) {
        None
    } else {
        Some(
            (((close.clone() / open.clone()) - bigdecimal::BigDecimal::from(1))
                * bigdecimal::BigDecimal::from(100))
            .with_scale_round(36, bigdecimal::RoundingMode::HalfEven)
            .normalized()
            .to_plain_string(),
        )
    };
    Some(
        json!({"first_timestamp":first["timestamp"],"last_timestamp":last["timestamp"],"open":open.normalized().to_plain_string(),"high":highs.into_iter().max().map(|v|v.normalized().to_plain_string()),"low":lows.into_iter().min().map(|v|v.normalized().to_plain_string()),"close":close.normalized().to_plain_string(),"volume_usd":volumes.into_iter().sum::<bigdecimal::BigDecimal>().normalized().to_plain_string(),"change_pct":change}),
    )
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TradesArgs {
    /// Exact Robinhood Chain ERC-20 0x contract address. Do not pass a symbol
    /// or name; resolve those with hoodit_search_tokens first.
    pub token: String,
    /// Optional opaque pool_id returned by a Hoodit result. Omit to use the
    /// token's indexed top pool. Trades cover only the selected pool.
    #[serde(default)]
    #[schemars(
        with = "String",
        length(min = 1, max = 200),
        pattern(r"^[A-Za-z0-9:_-]+$")
    )]
    pub pool_id: Option<String>,
    /// Maximum trades to return after filtering, from 1 through 100. Omit for
    /// 20.
    #[serde(default)]
    #[schemars(with = "u16", range(min = 1, max = 100), extend("default" = 20))]
    pub limit: Option<u16>,
    /// Minimum per-trade USD volume as a non-negative decimal string, for
    /// example "1000". Omit for "0".
    #[serde(default)]
    #[schemars(with = "String", pattern(r"^(0|[1-9][0-9]*)(\.[0-9]+)?$"), extend("default" = "0"))]
    pub min_volume_usd: Option<String>,
}
pub struct GetTrades;
impl DynAomiTool for GetTrades {
    type App = HooditApp;
    type Args = TradesArgs;
    const NAME: &'static str = "hoodit_get_trades";
    const DESCRIPTION: &'static str = "Read recent public trades for one exact token contract in one selected pool, with buy/sell side normalized to that token. Requires a 0x contract address; this is public pool activity, not the user's personal history.";
    fn run(app: &HooditApp, args: TradesArgs, _: DynToolCallCtx) -> Result<Value, String> {
        let token = invalid!(model::address(&args.token));
        let limit = args.limit.unwrap_or(20);
        if !(1..=100).contains(&limit) {
            return Ok(model::error(
                "INVALID_ARGUMENT",
                "limit must be 1 to 100",
                false,
            ));
        }
        let min = invalid!(model::decimal(
            args.min_volume_usd.as_deref().unwrap_or("0")
        ));
        let explicit = invalid!(args.pool_id.as_deref().map(pool_id).transpose());
        let runtime = app.runtime()?;
        let gecko = Gecko::new(&runtime);
        let mut read = ReadContext::markets(false);
        let (pool, id) = match selected_pool(&gecko, &token, explicit.as_deref(), &mut read) {
            Ok(v) => v,
            Err(e) => return Ok(provider_error(e)),
        };
        let raw = match gecko.trades(&id, &token, &min, &mut read) {
            Ok(v) => v,
            Err(e) => return Ok(provider_error(e)),
        };
        let cutoff = Utc::now().timestamp() - 86_400;
        let mut trades=data_array(&raw).into_iter().map(|v|{let a=v.get("attributes").cloned().unwrap_or(json!({}));let from=model::string(&a,&["from_token_address"]).unwrap_or_default();let to=model::string(&a,&["to_token_address"]).unwrap_or_default();let (side,token_amount,counter,counter_amount,price)=if to.eq_ignore_ascii_case(&token){("buy",model::string(&a,&["to_token_amount"]),from,model::string(&a,&["from_token_amount"]),model::string(&a,&["price_to_in_usd"]))}else if from.eq_ignore_ascii_case(&token){("sell",model::string(&a,&["from_token_amount"]),to,model::string(&a,&["to_token_amount"]),model::string(&a,&["price_from_in_usd"]))}else{("unknown",None,String::new(),None,None)};let timestamp=model::string(&a,&["block_timestamp"]).and_then(|s|chrono::DateTime::parse_from_rfc3339(&s).ok()).map(|t|t.timestamp());json!({"id":model::string(&v,&["id"]),"tx_hash":model::string(&a,&["tx_hash"]),"timestamp":timestamp,"side":side,"token_amount":token_amount,"counter_token":if counter.is_empty(){None}else{Some(counter)},"counter_amount":counter_amount,"price_usd":price,"volume_usd":model::string(&a,&["volume_in_usd"])})}).filter(|v|v["timestamp"].as_i64().is_some_and(|ts|ts>=cutoff)&&decimal_ge(model::string(v,&["volume_usd"]).as_deref(),&min)).collect::<Vec<_>>();
        trades.sort_by_key(|v| std::cmp::Reverse(v["timestamp"].as_i64().unwrap_or_default()));
        trades.truncate(limit as usize);
        let returned = trades.len();
        Ok(model::ok(
            json!({"token":token,"pool":{"pool_id":pool["pool_id"],"dex_id":pool["dex_id"],"dex_name":pool["dex_name"]},"trades":trades,"coverage":{"scope":"single_pool","lookback_seconds":86400,"provider_trade_cap":300,"returned":returned,"complete_history":false}}),
            read.sources,
            read.warnings,
        ))
    }
}
