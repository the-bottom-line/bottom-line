use game::player::{CardType, Character};
use responses;
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

    #[wasm_bindgen(js_name = endTurn)]
    pub fn end_turn() -> Result<String, JsValue> {
        serde_json::to_string(&responses::FrontendRequest::EndTurn)
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
            responses::FrontendRequest::EndTurn => Self::end_turn(),
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
