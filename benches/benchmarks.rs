use bottom_line::{
    game::{GameState, TheBottomLine},
    player::PlayerId,
};
use claim::assert_matches;
use diol::prelude::*;

fn get_gamestate(player_count: usize) -> GameState {
    let mut game = GameState::new();

    (0..player_count)
        .map(|i| format!("Player {i}"))
        .for_each(|name| assert_matches!(game.join(name), Ok(true)));

    game.start_game("assets/cards/boardgame.json").unwrap();

    game
}

fn main() -> std::io::Result<()> {
    let mut bench = Bench::new(BenchConfig::from_args()?);

    bench.register(player_info, 4..=7);
    bench.register(get_selectable_characters, 0..4);

    bench.run()?;

    Ok(())
}

fn player_info(bencher: Bencher, player_count: usize) {
    let state = get_gamestate(player_count);

    bencher.bench(|| state.player_info(1.into()))
}

fn get_selectable_characters(bencher: Bencher, player_id: u8) {
    let state = get_gamestate(7);

    bencher.bench(|| state.player_get_selectable_characters(PlayerId::from(player_id)))
}
