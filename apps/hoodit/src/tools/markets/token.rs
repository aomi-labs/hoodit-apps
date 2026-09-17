use super::normalization::{invalid_argument, normalize_pool_id, resolve_pool, response_rows};
use crate::{
    app::{HooditApp, ReadContext},
    model,
    providers::{Gecko, included_map, pool, token_from_resource},
    tools::provider_error,
};
use aomi_sdk::schemars::JsonSchema;
use aomi_sdk::{DynAomiTool, DynToolCallCtx};
use serde::Deserialize;
use serde_json::{Value, json};

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
        let token = invalid_argument!(model::address(&args.token));
        let explicit_pool_id =
            invalid_argument!(args.pool_id.as_deref().map(normalize_pool_id).transpose());
        let runtime = app.runtime()?;
        let gecko = Gecko::new(&runtime);
        let mut read = ReadContext::markets(false);
        let token_response = match gecko.token(&token, &mut read) {
            Ok(response) => response,
            Err(error) => return Ok(provider_error(error)),
        };
        let Some(token_resource) = response_rows(&token_response).into_iter().next() else {
            return Ok(model::error(
                "UPSTREAM_SCHEMA_CHANGED",
                "token response is empty",
                false,
            ));
        };
        let attributes = token_resource
            .get("attributes")
            .cloned()
            .unwrap_or(json!({}));
        let token_details = token_from_resource(&token_resource);
        let mut warnings = vec![];
        let top_pool_ids = model::get(&token_resource, &["relationships", "top_pools", "data"])
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|pool| model::string(pool, &["id"]))
            .map(|id| id.strip_prefix("robinhood_").unwrap_or(&id).to_string())
            .collect::<Vec<_>>();
        let automatic_pool_id = top_pool_ids.first().map(String::as_str);
        let selected_pool = match resolve_pool(
            &gecko,
            &token,
            explicit_pool_id.as_deref().or(automatic_pool_id),
            &mut read,
        ) {
            Ok((pool, _)) => Some(pool),
            Err(error) if explicit_pool_id.is_none() => {
                warnings.push(model::warning(
                    if error.code == "NO_INDEXED_POOL" {
                        "NOT_INDEXED"
                    } else {
                        "METADATA_UNAVAILABLE"
                    },
                    if error.code == "NO_INDEXED_POOL" {
                        "No indexed reference pool was found"
                    } else {
                        "Token detail is available, but selected-pool context could not be read"
                    },
                ));
                None
            }
            Err(error) => return Ok(provider_error(error)),
        };
        let metadata = if args.include_metadata.unwrap_or(false) {
            match gecko.metadata(&token, &mut read) {
                Ok(response) => response_rows(&response)
                    .into_iter()
                    .next()
                    .map(|resource| normalize_metadata(&resource)),
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
        let selected_token_price_usd = selected_pool.as_ref().and_then(|pool| {
            if model::string(pool, &["base_token", "id"]).as_deref() == Some(token.as_str()) {
                pool.get("base_price_usd").cloned()
            } else {
                pool.get("quote_price_usd").cloned()
            }
        });
        let selected_pool_id = selected_pool
            .as_ref()
            .and_then(|pool| model::string(pool, &["pool_id"]));
        let mut other_pools = Vec::new();
        for pool_id in top_pool_ids
            .iter()
            .filter(|pool_id| Some(pool_id.as_str()) != selected_pool_id.as_deref())
            .take(4)
        {
            if let Ok(response) = gecko.pool(pool_id, &mut read) {
                let included = included_map(&response);
                if let Some(row) = response_rows(&response).into_iter().next() {
                    let pool = pool(&row, &included);
                    other_pools.push(json!({"pool_id":pool["pool_id"],"dex_id":pool["dex_id"],"dex_name":pool["dex_name"]}));
                }
            }
        }
        Ok(model::ok(
            json!({"token":token_details,"price_usd":model::string(&attributes,&["price_usd"]),"market_cap_usd":model::string(&attributes,&["market_cap_usd"]),"fdv_usd":model::string(&attributes,&["fdv_usd"]),"volume_24h_usd":model::string(&attributes,&["volume_usd","h24"]),"selected_pool":selected_pool,"selected_token_price_usd":selected_token_price_usd,"other_pools":other_pools,"metadata":metadata}),
            read.sources,
            {
                warnings.extend(read.warnings);
                warnings
            },
        ))
    }
}

fn normalize_metadata(resource: &Value) -> Value {
    let attributes = resource.get("attributes").unwrap_or(resource);
    let description = model::string(attributes, &["description"])
        .map(|description| description.chars().take(1000).collect::<String>());
    let websites = attributes
        .get("websites")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .take(5)
        .collect::<Vec<_>>();
    json!({"description":description,"websites":websites,"twitter":model::string(attributes,&["twitter_handle"]).or_else(||model::string(attributes,&["twitter_url"])),"telegram":model::string(attributes,&["telegram_handle"]).or_else(||model::string(attributes,&["telegram_url"])),"discord":model::string(attributes,&["discord_url"])})
}
