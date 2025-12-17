use game::player::{CardType, Character, Color, PlayerId};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct CreateRequest;

#[wasm_bindgen]
impl CreateRequest {
    #[wasm_bindgen(js_name = connect)]
    pub fn connect(username: &str, lobby: &str) -> Result<String, JsValue> {
        let username = username.to_owned();
        let channel = lobby.to_owned();
        serde_json::to_string(&responses::Connect::Connect { username, channel })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = startGame)]
    pub fn start_game() -> Result<String, JsValue> {
        serde_json::to_string(&responses::FrontendRequest::StartGame)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = selectCharacter)]
    pub fn select_character(character: JsValue) -> Result<String, JsValue> {
        let character: Character = serde_wasm_bindgen::from_value(character)?;
        serde_json::to_string(&responses::FrontendRequest::SelectCharacter { character })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = drawCard)]
    pub fn draw_card(card_type: JsValue) -> Result<String, JsValue> {
        let card_type: CardType = serde_wasm_bindgen::from_value(card_type)?;
        serde_json::to_string(&responses::FrontendRequest::DrawCard { card_type })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = putBackCard)]
    pub fn put_back_card(card_idx: usize) -> Result<String, JsValue> {
        serde_json::to_string(&responses::FrontendRequest::PutBackCard { card_idx })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = buyAsset)]
    pub fn buy_asset(card_idx: usize) -> Result<String, JsValue> {
        serde_json::to_string(&responses::FrontendRequest::BuyAsset { card_idx })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = issueLiability)]
    pub fn issue_liability(card_idx: usize) -> Result<String, JsValue> {
        serde_json::to_string(&responses::FrontendRequest::IssueLiability { card_idx })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = redeemLiability)]
    pub fn redeem_liability(liability_idx: usize) -> Result<String, JsValue> {
        serde_json::to_string(&responses::FrontendRequest::RedeemLiability { liability_idx })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = useAbility)]
    pub fn use_ability() -> Result<String, JsValue> {
        serde_json::to_string(&responses::FrontendRequest::UseAbility)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = fireCharacter)]
    pub fn fire_character(character: JsValue) -> Result<String, JsValue> {
        let character: Character = serde_wasm_bindgen::from_value(character)?;
        serde_json::to_string(&responses::FrontendRequest::FireCharacter { character })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = terminateCreditCharacter)]
    pub fn terminate_credit_character(character: JsValue) -> Result<String, JsValue> {
        let character: Character = serde_wasm_bindgen::from_value(character)?;
        serde_json::to_string(&responses::FrontendRequest::TerminateCreditCharacter { character })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
    #[wasm_bindgen(js_name = payBanker)]
    pub fn pay_banker(cash: u8) -> Result<String, JsValue> {
        serde_json::to_string(&responses::FrontendRequest::PayBanker { cash })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
    #[wasm_bindgen(js_name = selectAssetToDivest)]
    pub fn select_asset_to_divest(asset_id: usize) -> Result<String, JsValue> {
        serde_json::to_string(&responses::FrontendRequest::SelectAssetToDivest { asset_id })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
    #[wasm_bindgen(js_name = unselectAssetToDivest)]
    pub fn unselect_asset_to_divest(asset_id: usize) -> Result<String, JsValue> {
        serde_json::to_string(&responses::FrontendRequest::UnselectAssetToDivest { asset_id })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
    #[wasm_bindgen(js_name = selectLiabilityToIssue)]
    pub fn select_liability_to_issue(liability_id: usize) -> Result<String, JsValue> {
        serde_json::to_string(&responses::FrontendRequest::SelectLiabilityToIssue { liability_id })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
    #[wasm_bindgen(js_name = unselectLiabilityToIssue)]
    pub fn unselect_liability_to_issue(liability_id: usize) -> Result<String, JsValue> {
        serde_json::to_string(&responses::FrontendRequest::UnselectLiabilityToIssue {
            liability_id,
        })
        .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = swapWithDeck)]
    pub fn swap_with_deck(card_idxs: Vec<usize>) -> Result<String, JsValue> {
        serde_json::to_string(&responses::FrontendRequest::SwapWithDeck { card_idxs })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = swapWithPlayer)]
    pub fn swap_with_player(target_id: u8) -> Result<String, JsValue> {
        let target_player_id = PlayerId(target_id);
        serde_json::to_string(&responses::FrontendRequest::SwapWithPlayer { target_player_id })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = divestAsset)]
    pub fn divest_asset(target_id: u8, asset_idx: usize) -> Result<String, JsValue> {
        let target_player_id = PlayerId(target_id);
        serde_json::to_string(&responses::FrontendRequest::DivestAsset {
            target_player_id,
            card_idx: asset_idx,
        })
        .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = endTurn)]
    pub fn end_turn() -> Result<String, JsValue> {
        serde_json::to_string(&responses::FrontendRequest::EndTurn)
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = minusIntoPlus)]
    pub fn minus_into_plus(color: JsValue) -> Result<String, JsValue> {
        let color: Color = serde_wasm_bindgen::from_value(color)?;
        serde_json::to_string(&responses::FrontendRequest::MinusIntoPlus { color })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = silverIntoGold)]
    pub fn silver_into_gold(asset_idx: usize) -> Result<String, JsValue> {
        serde_json::to_string(&responses::FrontendRequest::SilverIntoGold { asset_idx })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = changeAssetColor)]
    pub fn change_asset_color(asset_idx: usize, color: JsValue) -> Result<String, JsValue> {
        let color: Color = serde_wasm_bindgen::from_value(color)?;
        serde_json::to_string(&responses::FrontendRequest::ChangeAssetColor { asset_idx, color })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }

    #[wasm_bindgen(js_name = confirmAssetAbility)]
    pub fn confirm_asset_ability(asset_idx: usize) -> Result<String, JsValue> {
        serde_json::to_string(&responses::FrontendRequest::ConfirmAssetAbility { asset_idx })
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
}

impl CreateRequest {
    fn _typecheck(connect: responses::Connect, request: responses::FrontendRequest) {
        let _ = match connect {
            responses::Connect::Connect { username, channel } => Self::connect(&username, &channel),
        };
        let _ = match request {
            responses::FrontendRequest::StartGame => Self::start_game(),
            responses::FrontendRequest::SelectCharacter { .. } => {
                Self::select_character(JsValue::null())
            }
            responses::FrontendRequest::DrawCard { .. } => Self::draw_card(JsValue::null()),
            responses::FrontendRequest::PutBackCard { card_idx } => Self::put_back_card(card_idx),
            responses::FrontendRequest::BuyAsset { card_idx } => Self::buy_asset(card_idx),
            responses::FrontendRequest::IssueLiability { card_idx } => {
                Self::issue_liability(card_idx)
            }
            responses::FrontendRequest::RedeemLiability { liability_idx } => {
                Self::redeem_liability(liability_idx)
            }
            responses::FrontendRequest::UseAbility => Self::use_ability(),
            responses::FrontendRequest::FireCharacter { .. } => {
                Self::fire_character(JsValue::null())
            }
            responses::FrontendRequest::TerminateCreditCharacter { .. } => {
                Self::terminate_credit_character(JsValue::null())
            }
            responses::FrontendRequest::PayBanker { cash } => Self::pay_banker(cash),
            responses::FrontendRequest::SelectAssetToDivest { asset_id } => {
                Self::select_asset_to_divest(asset_id)
            }
            responses::FrontendRequest::UnselectAssetToDivest { asset_id } => {
                Self::unselect_asset_to_divest(asset_id)
            }
            responses::FrontendRequest::SelectLiabilityToIssue { liability_id } => {
                Self::select_liability_to_issue(liability_id)
            }
            responses::FrontendRequest::UnselectLiabilityToIssue { liability_id } => {
                Self::unselect_liability_to_issue(liability_id)
            }
            responses::FrontendRequest::SwapWithDeck { card_idxs } => {
                Self::swap_with_deck(card_idxs)
            }
            responses::FrontendRequest::SwapWithPlayer { target_player_id } => {
                Self::swap_with_player(target_player_id.0)
            }
            responses::FrontendRequest::DivestAsset {
                target_player_id,
                card_idx,
            } => Self::divest_asset(target_player_id.0, card_idx),
            responses::FrontendRequest::EndTurn => Self::end_turn(),
            responses::FrontendRequest::MinusIntoPlus { color: _ } => {
                Self::minus_into_plus(JsValue::null())
            }
            responses::FrontendRequest::SilverIntoGold { asset_idx } => {
                Self::silver_into_gold(asset_idx)
            }
            responses::FrontendRequest::ChangeAssetColor {
                asset_idx,
                color: _,
            } => Self::change_asset_color(asset_idx, JsValue::null()),
            responses::FrontendRequest::ConfirmAssetAbility { asset_idx } => {
                Self::confirm_asset_ability(asset_idx)
            }
        };
    }
}

// enabling this costs like 270kB

// #[wasm_bindgen]
// pub struct DirectResponseParser;

// #[wasm_bindgen]
// impl DirectResponseParser {
//     #[wasm_bindgen(js_name = parseJson)]
//     pub fn parse_json(json: &str) -> Result<JsValue, JsValue> {
//         let response: responses::DirectResponse =
//             serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
//         serde_wasm_bindgen::to_value(&response).map_err(|e| JsValue::from_str(&e.to_string()))
//     }
// }

// #[wasm_bindgen]
// pub struct UniqueResponseParser;

// #[wasm_bindgen]
// impl UniqueResponseParser {
//     #[wasm_bindgen(js_name = parseJson)]
//     pub fn parse_json(json: &str) -> Result<JsValue, JsValue> {
//         let response: responses::UniqueResponse =
//             serde_json::from_str(json).map_err(|e| JsValue::from_str(&e.to_string()))?;
//         serde_wasm_bindgen::to_value(&response).map_err(|e| JsValue::from_str(&e.to_string()))
//     }
// }

#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
}
