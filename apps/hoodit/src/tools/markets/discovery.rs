use super::normalization::{decimal_at_least, invalid_argument, response_rows, validate_page};
use crate::{
    app::{HooditApp, ReadContext},
    model,
    providers::{Gecko, included_map, pool},
    tools::provider_error,
};
use aomi_sdk::schemars::JsonSchema;
use aomi_sdk::{DynAomiTool, DynToolCallCtx};
use serde::Deserialize;
use serde_json::{Value, json};

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

        let page = invalid_argument!(validate_page(args.page));
        let min_liquidity_usd = invalid_argument!(model::decimal(
            args.min_liquidity_usd.as_deref().unwrap_or("0")
        ));
        let min_volume_24h_usd = invalid_argument!(model::decimal(
            args.min_volume_24h_usd.as_deref().unwrap_or("0")
        ));
        let runtime = app.runtime()?;
        let mut read = ReadContext::markets(false);
        let response = match Gecko::new(&runtime).discover(feed, duration, page, &mut read) {
            Ok(response) => response,
            Err(error) => return Ok(provider_error(error)),
        };
        let included = included_map(&response);
        let rows = response_rows(&response);
        let response_page_full = rows.len() >= 20;
        let pools = rows
            .iter()
            .map(|row| pool(row, &included))
            .filter(|pool| {
                decimal_at_least(
                    model::string(pool, &["liquidity_usd"]).as_deref(),
                    &min_liquidity_usd,
                ) && decimal_at_least(
                    model::string(pool, &["windows", "h24", "volume_usd"]).as_deref(),
                    &min_volume_24h_usd,
                )
            })
            .collect::<Vec<_>>();
        let returned = pools.len();
        let next_page = if page < 10 && response_page_full {
            Some(page + 1)
        } else {
            None
        };
        let mut warnings = read.warnings;
        if page == 10 && response_page_full {
            warnings.push(model::warning(
                "PARTIAL_PAGE",
                "The feed reached its page limit while more pools may exist",
            ));
        }
        Ok(model::ok(
            json!({"feed":feed,"duration":if feed=="trending"{Some(duration)}else{None},"min_liquidity_usd":min_liquidity_usd,"min_volume_24h_usd":min_volume_24h_usd,"pools":pools,"pagination":{"page":page,"page_size":20,"returned":returned,"next_page":next_page}}),
            read.sources,
            warnings,
        ))
    }
}
