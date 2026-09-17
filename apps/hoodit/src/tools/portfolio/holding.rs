use super::valuation::{HoldingInput, make_holding, quote_token, verify_usdg_token};
use crate::{
    amount,
    app::{HooditApp, ReadContext},
    model,
    providers::{Blockscout, Lifi, ProviderError},
    tools::provider_error,
};
use aomi_sdk::schemars::JsonSchema;
use aomi_sdk::{DynAomiTool, DynToolCallCtx};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HoldingArgs {
    /// Exact public 0x wallet address on Robinhood Chain. For "my wallet",
    /// resolve the funded executor with get_account_info on chain 4663 first.
    pub wallet_address: String,
    /// Exact ERC-20 0x contract address, or the literal "native" for native
    /// ETH. Do not pass a ticker symbol or token name.
    pub token: String,
    /// Portion of the holding to size and optionally quote, in basis points:
    /// 1 = 0.01%, 100 = 1%, and 10000 = 100%. Omit for 100 (1%).
    #[serde(default)]
    #[schemars(with = "u16", range(min = 1, max = 10000), extend("default" = 100))]
    pub quote_balance_bps: Option<u16>,
    /// Request a read-only LI.FI estimate for the sized amount into USDG.
    /// Omit for false when only the exact balance or amount is needed.
    #[serde(default)]
    #[schemars(with = "bool", extend("default" = false))]
    pub include_quote: Option<bool>,
    /// Bypass Hoodit's short-lived read cache. Omit to use the default true for
    /// this exact holding read; pass false only when cached data is acceptable.
    #[serde(default)]
    #[schemars(with = "bool", extend("default" = true))]
    pub refresh: Option<bool>,
}

pub struct GetHolding;

impl DynAomiTool for GetHolding {
    type App = HooditApp;
    type Args = HoldingArgs;
    const NAME: &'static str = "hoodit_get_holding";
    const DESCRIPTION: &'static str = "Read one exact wallet holding and floor-size a requested fraction. Use token=\"native\" for native ETH or an exact ERC-20 0x contract address; quote_balance_bps is a percentage in basis points. A quote is read-only and is included only when include_quote=true.";

    fn run(app: &HooditApp, args: HoldingArgs, ctx: DynToolCallCtx) -> Result<Value, String> {
        let mut read = ReadContext::portfolio(args.refresh.unwrap_or(true));
        let wallet = match model::address(&args.wallet_address) {
            Ok(wallet) => wallet,
            Err(message) => return Ok(model::error("INVALID_ARGUMENT", &message, false)),
        };
        let token = match model::token_id(&args.token) {
            Ok(token) => token,
            Err(message) => return Ok(model::error("INVALID_ARGUMENT", &message, false)),
        };
        let quote_balance_bps = args.quote_balance_bps.unwrap_or(100);
        if !(1..=10_000).contains(&quote_balance_bps) {
            return Ok(model::error(
                "INVALID_ARGUMENT",
                "quote_balance_bps must be 1 to 10000",
                false,
            ));
        }
        let runtime = app.runtime()?;
        let blockscout = match Blockscout::from_ctx(&runtime, &ctx) {
            Ok(blockscout) => blockscout,
            Err(error) => return Ok(provider_error(error)),
        };
        let include_quote = args.include_quote.unwrap_or(false);
        if include_quote && let Err(error) = verify_usdg_token(&blockscout, &mut read) {
            return Ok(provider_error(error));
        }
        let (raw_balance, symbol, name, decimals) = if token == "native" {
            match blockscout.native_balance(&wallet, &mut read) {
                Ok(balance) => (balance, Some("ETH".into()), Some("Ether".into()), Some(18)),
                Err(error) => return Ok(provider_error(error)),
            }
        } else {
            let raw_balance = match blockscout.token_balance(&wallet, &token, &mut read) {
                Ok(balance) => balance,
                Err(error) => return Ok(provider_error(error)),
            };
            let mut token_info = blockscout.token_info(&token, &mut read).ok();
            if let Some(info) = token_info.as_ref() {
                match model::string(info, &["address_hash"]) {
                    Some(address) if address.eq_ignore_ascii_case(&token) => {}
                    Some(_) => {
                        return Ok(provider_error(ProviderError {
                            code: "UPSTREAM_SCHEMA_CHANGED",
                            message:
                                "Blockscout token metadata identity does not match the request"
                                    .into(),
                            retryable: false,
                        }));
                    }
                    None => token_info = None,
                }
            }
            (
                raw_balance,
                token_info
                    .as_ref()
                    .and_then(|info| model::string(info, &["symbol"])),
                token_info
                    .as_ref()
                    .and_then(|info| model::string(info, &["name"])),
                token_info
                    .as_ref()
                    .and_then(|info| model::string(info, &["decimals"]))
                    .and_then(|decimals| decimals.parse().ok()),
            )
        };
        let balance = match amount::atomic(&raw_balance) {
            Ok(balance) => balance,
            Err(_) => {
                return Ok(model::error(
                    "UPSTREAM_SCHEMA_CHANGED",
                    "provider returned an invalid balance",
                    false,
                ));
            }
        };
        let sell_amount = amount::fraction(&balance, quote_balance_bps);
        let mut warnings = vec![];
        let lifi = Lifi::from_ctx(&runtime, &ctx);
        let mut quote_attempts = 0;
        let holding = match make_holding(
            HoldingInput {
                token_id: &token,
                symbol: symbol.as_deref(),
                name: name.as_deref(),
                decimals,
                raw_balance: &raw_balance,
                wallet: &wallet,
            },
            include_quote,
            quote_balance_bps,
            &mut quote_attempts,
            &lifi,
            &mut read,
            &mut warnings,
        ) {
            Ok(holding) => holding,
            Err(error) => return Ok(provider_error(error)),
        };
        let sell_amount_formatted = decimals.map(|decimals| amount::format(&sell_amount, decimals));
        Ok(model::ok(
            json!({"wallet_address":wallet,"quote_token":quote_token(include_quote),"holding":holding,"requested_balance_bps":quote_balance_bps,"sell_amount":{"atomic":sell_amount.to_string(),"formatted":sell_amount_formatted}}),
            read.sources,
            {
                warnings.extend(read.warnings);
                warnings
            },
        ))
    }
}
