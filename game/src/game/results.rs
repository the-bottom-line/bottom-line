//! File containing the results state of the game.

use crate::{errors::*, game::*, player::*};

/// State containing all information related to the results state of the game. In the resuts stage,
/// players can see their scores.
#[derive(Debug, Clone, PartialEq)]
pub struct Results {
    pub(super) players: Players<ResultsPlayer>,
    pub(super) final_market: Market,
    // TODO: implement events
    pub(super) final_events: Vec<Event>,
}

impl Results {
    /// Get a reference to a [`ResultsPlayer`] based on a specific `PlayerId`. Note that the players
    /// are in order, so id 0 refers to the player at index 0 and so on.
    /// See [`Players::player`] for further information
    pub fn player(&self, id: PlayerId) -> Result<&ResultsPlayer, GameError> {
        self.players.player(id)
    }

    /// Get a reference to a [`ResultsPlayer`] based on a specific `name`.
    pub fn player_by_name(&self, name: &str) -> Result<&ResultsPlayer, GameError> {
        self.players()
            .iter()
            .find(|p| p.name() == name)
            .ok_or_else(|| GameError::InvalidPlayerName(name.to_owned()))
    }

    /// Gets a slice of all players in the lobby.
    /// See [`Players::players`] for further information
    pub fn players(&self) -> &[ResultsPlayer] {
        self.players.players()
    }

    /// Returns a list of [`PlayerScore`], which contains the player id as well as their final
    /// score.
    pub fn player_scores(&self) -> Vec<PlayerScore> {
        self.players()
            .iter()
            .map(|p| PlayerScore::new(p.id(), p.name(), p.score()))
            .collect()
    }

    /// Gets the [`PlayerInfo`] for each player, excluding the player that has the same id as `id`.
    pub fn player_info(&self, id: PlayerId) -> Vec<PlayerInfo> {
        self.players()
            .iter()
            .filter(|p| p.id() != id)
            .map(Into::into)
            .collect()
    }

    /// Gets the final market of the game
    pub fn final_market(&self) -> &Market {
        &self.final_market
    }

    /// Gets the list of events that happened over the course of the game
    pub fn final_events(&self) -> &[Event] {
        &self.final_events
    }

    /// Increases one of the market conditions of a certain color for player with `id`. This means
    /// that minus is turned into zero and zero is turned into plus. Returns the resulting market.
    pub fn minus_into_plus(&mut self, id: PlayerId, color: Color) -> Result<Market, GameError> {
        let player = self.players.player_mut(id)?;
        let market = player.minus_into_plus(color, &self.final_market).to_owned();

        Ok(market)
    }

    /// Toggles the [`SilverIntoGold`] asset ability for a particular player.
    pub fn toggle_silver_into_gold(
        &mut self,
        id: PlayerId,
        asset_idx: usize,
    ) -> Result<ToggleSilverIntoGold, GameError> {
        let player = self.players.player_mut(id)?;
        let data = player.toggle_silver_into_gold(asset_idx)?;

        Ok(data)
    }

    /// Toggles the [`ChangeAssetColor`] asset ability for a particular player.
    pub fn toggle_change_asset_color(
        &mut self,
        id: PlayerId,
        asset_idx: usize,
        color: Color,
    ) -> Result<ToggleChangeAssetColor, GameError> {
        let player = self.players.player_mut(id)?;
        let data = player.toggle_change_asset_color(asset_idx, color)?;

        Ok(data)
    }

    /// Asset abilities are toggleable by default. This function confirms the current configuration
    /// for this particular player, after which they cannot toggle this particular index anymore.
    pub fn confirm_asset_ability(
        &mut self,
        id: PlayerId,
        asset_idx: usize,
    ) -> Result<(), GameError> {
        let player = self.players.player_mut(id)?;
        player.confirm_asset_ability(asset_idx)
    }
}

/// Representation of a player's final score, which contains their id as well as their score.
///
/// # Examples
///
/// ```
/// # use game::{game::PlayerScore, player::PlayerId};
/// let score = PlayerScore::new(PlayerId(0), "oxey", 10.0);
/// assert_eq!(score.id(), PlayerId(0));
/// assert_eq!(score.name(), "oxey");
/// assert_eq!(score.score(), 10.0);
/// ```
#[cfg_attr(feature = "ts", derive(TS))]
#[cfg_attr(feature = "ts", ts(export_to = crate::SHARED_TS_DIR))]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlayerScore {
    id: PlayerId,
    name: String,
    score: f64,
}

impl PlayerScore {
    /// Constructs a new [`PlayerScore`]
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::{game::PlayerScore, player::PlayerId};
    /// let score = PlayerScore::new(PlayerId(0), "oxey", 10.0);
    /// ```
    pub fn new(id: PlayerId, name: &str, score: f64) -> Self {
        let name = name.to_owned();
        Self { id, name, score }
    }

    /// Gets a [`PlayerScore`]'s `id` field.
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::{game::PlayerScore, player::PlayerId};
    /// let score = PlayerScore::new(PlayerId(0), "oxey", 10.0);
    /// assert_eq!(score.id(), PlayerId(0));
    /// ```
    pub fn id(&self) -> PlayerId {
        self.id
    }

    /// Gets a [`PlayerScore`]'s `name` field.
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::{game::PlayerScore, player::PlayerId};
    /// let score = PlayerScore::new(PlayerId(0), "oxey", 10.0);
    /// assert_eq!(score.name(), "oxey");
    /// ```
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Gets a [`PlayerScore`]'s `score` field.
    ///
    /// # Examples
    ///
    /// ```
    /// # use game::{game::PlayerScore, player::PlayerId};
    /// let score = PlayerScore::new(PlayerId(0), "oxey", 10.0);
    /// assert_eq!(score.score(), 10.0);
    /// ```
    pub fn score(&self) -> f64 {
        self.score
    }
}
