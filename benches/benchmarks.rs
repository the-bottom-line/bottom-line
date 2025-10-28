use bottom_line::{
    cards::GameData,
    game::{GameState, TheBottomLine},
};
use diol::prelude::*;

fn get_gamestate(player_count: usize) -> GameState {
    assert!((4..=7).contains(&player_count));
    
    let players = (0..player_count)
        .map(|i| format!("Player {i}"))
        .collect::<Vec<_>>();
    let data = GameData::new("assets/cards/boardgame.json").expect("this should exist");

    GameState::new(&players, data).unwrap()
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

    bencher.bench(|| state.player_info(1))
}

fn get_selectable_characters(bencher: Bencher, player_id: usize) {
    let state = get_gamestate(7);

    bencher.bench(|| state.player_get_selectable_characters(player_id))
}
