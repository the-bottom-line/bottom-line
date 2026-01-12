use game::{errors::GameError, game::GameState};
use responses::*;
use tokio::sync::broadcast;

use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

use crate::request_handler::*;

/// All-encompassing state each room has access to
pub struct RoomState {
    /// Internal broadcast that can be received by any connected thread
    pub tx: broadcast::Sender<UniqueResponse>,
    /// Internal broadcast channels to send responses specific to each player
    pub player_tx: [broadcast::Sender<UniqueResponse>; 7],
    /// Per-room gamestate
    pub game: Mutex<GameState>,
    /// Timestamp of last activity used for cleanup.
    pub last_activity: Arc<Mutex<Instant>>,
    /// A task that periodically checks if the room has been inactive and should be closed.
    pub cleanup_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl RoomState {
    pub fn new() -> Self {
        Self {
            tx: broadcast::channel(64).0,
            player_tx: [
                broadcast::channel(64).0,
                broadcast::channel(64).0,
                broadcast::channel(64).0,
                broadcast::channel(64).0,
                broadcast::channel(64).0,
                broadcast::channel(64).0,
                broadcast::channel(64).0,
            ],
            game: Mutex::new(GameState::new()),
            last_activity: Arc::new(Mutex::new(Instant::now())),
            cleanup_handle: Mutex::new(None),
        }
    }

    pub fn handle_request(
        &self,
        msg: FrontendRequest,
        player_name: &str,
    ) -> Result<Response, GameError> {
        self.touch();

        // PANIC: a mutex can only poison if any other thread that has access to it crashes. Since
        // this cannot happen, unwrapping is safe.
        let state = &mut *self.game.lock().unwrap();

        match msg {
            FrontendRequest::StartGame => start_game(state),
            FrontendRequest::SelectCharacter { character } => {
                // TODO: do something about this lookup madness
                let player_id = state
                    .selecting_characters()?
                    .player_by_name(player_name)?
                    .id();
                select_character(state, player_id, character)
            }
            FrontendRequest::DrawCard { card_type } => {
                let player_id = state.round()?.player_by_name(player_name)?.id();
                draw_card(state, card_type, player_id)
            }
            FrontendRequest::PutBackCard { card_idx } => {
                let player_id = state.round()?.player_by_name(player_name)?.id();
                put_back_card(state, card_idx, player_id)
            }
            FrontendRequest::BuyAsset { card_idx } => {
                let player_id = state.round()?.player_by_name(player_name)?.id();
                play_card(state, card_idx, player_id)
            }
            FrontendRequest::IssueLiability { card_idx } => {
                let player_id = state.round()?.player_by_name(player_name)?.id();
                play_card(state, card_idx, player_id)
            }
            FrontendRequest::RedeemLiability { liability_idx } => {
                let player_id = state.round()?.player_by_name(player_name)?.id();
                redeem_liability(state, liability_idx, player_id)
            }
            FrontendRequest::UseAbility => {
                let player_id = state.round()?.player_by_name(player_name)?.id();
                use_ability(state, player_id)
            }
            FrontendRequest::GetBonusCash => {
                let player_id = state.round()?.player_by_name(player_name)?.id();
                get_bonus_cash(state, player_id)
            }
            FrontendRequest::FireCharacter { character } => {
                let player_id = state.round()?.player_by_name(player_name)?.id();
                fire_character(state, player_id, character)
            }
            FrontendRequest::TerminateCreditCharacter { character } => {
                let player_id = state.round()?.player_by_name(player_name)?.id();
                terminate_credit_character(state, player_id, character)
            }
            FrontendRequest::SelectAssetToDivest { asset_id } => {
                let player_id = state.bankertarget()?.player_by_name(player_name)?.id();
                select_divest_asset(state, player_id, asset_id)
            }
            FrontendRequest::UnselectAssetToDivest { asset_id } => {
                let player_id = state.bankertarget()?.player_by_name(player_name)?.id();
                unselect_divest_asset(state, player_id, asset_id)
            }
            FrontendRequest::SelectLiabilityToIssue { liability_id } => {
                let player_id = state.bankertarget()?.player_by_name(player_name)?.id();
                select_issue_liability(state, player_id, liability_id)
            }
            FrontendRequest::UnselectLiabilityToIssue { liability_id } => {
                let player_id = state.bankertarget()?.player_by_name(player_name)?.id();
                unselect_issue_liability(state, player_id, liability_id)
            }
            FrontendRequest::PayBanker { cash } => {
                let player_id = state.bankertarget()?.player_by_name(player_name)?.id();
                pay_banker(state, player_id, cash)
            }
            FrontendRequest::SwapWithDeck { card_idxs } => {
                let player_id = state.round()?.player_by_name(player_name)?.id();
                swap_with_deck(state, player_id, card_idxs)
            }
            FrontendRequest::SwapWithPlayer { target_player_id } => {
                let player_id = state.round()?.player_by_name(player_name)?.id();
                swap_with_player(state, player_id, target_player_id)
            }
            FrontendRequest::DivestAsset {
                target_player_id,
                card_idx,
            } => {
                let player_id = state.round()?.player_by_name(player_name)?.id();
                divest_asset(state, player_id, target_player_id, card_idx)
            }
            FrontendRequest::EndTurn => {
                let player_id = state.round()?.player_by_name(player_name)?.id();
                end_turn(state, player_id)
            }
            FrontendRequest::MinusIntoPlus { color } => {
                let player_id = state.results()?.player_by_name(player_name)?.id();
                minus_into_plus(state, player_id, color)
            }
            FrontendRequest::SilverIntoGold { asset_idx } => {
                let player_id = state.results()?.player_by_name(player_name)?.id();
                silver_into_gold(state, player_id, asset_idx)
            }
            FrontendRequest::ChangeAssetColor { asset_idx, color } => {
                let player_id = state.results()?.player_by_name(player_name)?.id();
                change_asset_color(state, player_id, asset_idx, color)
            }
            FrontendRequest::ConfirmAssetAbility { asset_idx } => {
                let player_id = state.results()?.player_by_name(player_name)?.id();
                confirm_asset_ability(state, player_id, asset_idx)
            }
        }
    }

    /// Updates the timestamp the last action was taken in.
    pub fn touch(&self) {
        *self.last_activity.lock().unwrap() = Instant::now();
    }
}

impl Default for RoomState {
    fn default() -> Self {
        Self::new()
    }
}
