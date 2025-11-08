use bottom_line::{game::GameState, player::PlayerId};
use claim::assert_matches;
use diol::prelude::*;

fn get_gamestate(player_count: usize) -> GameState {
    let mut game = GameState::new();
    let lobby = game.lobby_mut().expect("game not in round state");

    (0..(player_count as u8))
        .map(|i| (i, format!("Player {i}")))
        .for_each(|(i, name)| assert_matches!(lobby.join(name), Ok(p) if p.id == PlayerId(i)));

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
    let mut state = get_gamestate(player_count);
    let selecting = state
        .selecting_characters_mut()
        .expect("not selecting characters");

    bencher.bench(|| selecting.player_info(1.into()))
}

fn get_selectable_characters(bencher: Bencher, player_id: u8) {
    let state = get_gamestate(7);
    let selecting = state.selecting_characters().unwrap();

    bencher.bench(|| selecting.player_get_selectable_characters(PlayerId::from(player_id)))
}
