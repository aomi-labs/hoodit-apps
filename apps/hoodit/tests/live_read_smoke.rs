use aomi_sdk::{DynAomiTool, DynToolCallCtx};
use hoodit::{app::HooditApp, tools::*};
use serde_json::json;
use std::collections::HashMap;

fn ctx(name: &str, key: &str) -> DynToolCallCtx {
    DynToolCallCtx {
        session_id: "hoodit-live-read-smoke".into(),
        tool_name: name.into(),
        call_id: format!("{name}-1"),
        state_attributes: Default::default(),
        secrets: HashMap::from([("BLOCKSCOUT_API_KEY".into(), key.into())]),
    }
}

#[test]
#[ignore = "read-only live provider smoke; set HOODIT_BLOCKSCOUT_API_KEY"]
fn live_read_tools_emit_envelopes() {
    let key =
        std::env::var("HOODIT_BLOCKSCOUT_API_KEY").expect("HOODIT_BLOCKSCOUT_API_KEY is required");
    let app = HooditApp::default();
    let token = std::env::var("HOODIT_LIVE_TOKEN")
        .unwrap_or_else(|_| "0x39dbed3a2bd333467115de45665cc57f813c4571".into());
    let pool = "0x4be9657ec9002e528f4f17a5c43edc525a07f888f7b180c2afbf75e096c4f38a";
    let wallet = std::env::var("HOODIT_LIVE_WALLET")
        .unwrap_or_else(|_| "0xb202bb725c85b90bd847d350ebc7f16ff8408ed8".into());
    let selected = std::env::var("HOODIT_LIVE_TOOL").unwrap_or_else(|_| "portfolio".into());
    let case = match selected.as_str() {
        "search" => {
            let input = json!({"query":token});
            (
                "hoodit_search_tokens",
                input.clone(),
                SearchTokens::run(
                    &app,
                    serde_json::from_value(input).unwrap(),
                    ctx("hoodit_search_tokens", &key),
                )
                .unwrap(),
            )
        }
        "discover" => {
            let input = json!({"feed":"trending","duration":"1h"});
            (
                "hoodit_discover_pools",
                input.clone(),
                DiscoverPools::run(
                    &app,
                    serde_json::from_value(input).unwrap(),
                    ctx("hoodit_discover_pools", &key),
                )
                .unwrap(),
            )
        }
        "token" => {
            let input = json!({"token":token,"pool_id":pool});
            (
                "hoodit_get_token",
                input.clone(),
                GetToken::run(
                    &app,
                    serde_json::from_value(input).unwrap(),
                    ctx("hoodit_get_token", &key),
                )
                .unwrap(),
            )
        }
        "candles" => {
            let input = json!({"token":token,"pool_id":pool,"interval":"15m","limit":5});
            (
                "hoodit_get_candles",
                input.clone(),
                GetCandles::run(
                    &app,
                    serde_json::from_value(input).unwrap(),
                    ctx("hoodit_get_candles", &key),
                )
                .unwrap(),
            )
        }
        "trades" => {
            let input = json!({"token":token,"pool_id":pool,"limit":5});
            (
                "hoodit_get_trades",
                input.clone(),
                GetTrades::run(
                    &app,
                    serde_json::from_value(input).unwrap(),
                    ctx("hoodit_get_trades", &key),
                )
                .unwrap(),
            )
        }
        "holding" => {
            let input =
                json!({"wallet_address":wallet,"token":token,"include_quote":false,"refresh":true});
            (
                "hoodit_get_holding",
                input.clone(),
                GetHolding::run(
                    &app,
                    serde_json::from_value(input).unwrap(),
                    ctx("hoodit_get_holding", &key),
                )
                .unwrap(),
            )
        }
        "holding_quote" => {
            let input = json!({"wallet_address":wallet,"token":token,"include_quote":true,"quote_balance_bps":10000,"refresh":true});
            (
                "hoodit_get_holding",
                input.clone(),
                GetHolding::run(
                    &app,
                    serde_json::from_value(input).unwrap(),
                    ctx("hoodit_get_holding", &key),
                )
                .unwrap(),
            )
        }
        _ => {
            let input = json!({"wallet_address":wallet,"include_quotes":false,"refresh":true});
            (
                "hoodit_get_portfolio",
                input.clone(),
                GetPortfolio::run(
                    &app,
                    serde_json::from_value(input).unwrap(),
                    ctx("hoodit_get_portfolio", &key),
                )
                .unwrap(),
            )
        }
    };
    let cases = vec![case];
    for (tool, _, output) in &cases {
        assert!(
            matches!(output["status"].as_str(), Some("ok" | "partial")),
            "{tool}: {output}"
        );
    }
    let path = std::env::var("HOODIT_LIVE_OUTPUT")
        .unwrap_or_else(|_| "/tmp/hoodit-live-read-smoke.json".into());
    std::fs::write(path,serde_json::to_vec_pretty(&json!({"cases":cases.into_iter().map(|(tool,input,output)|json!({"tool":tool,"input":input,"output":output})).collect::<Vec<_>>()})).unwrap()).unwrap();
}
