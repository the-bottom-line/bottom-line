mod cards;
mod game;

use game::*;

use crate::cards::GameData;

fn main() -> anyhow::Result<()> {
    let data = GameData::new("assets/cards/boardgame.json")?;
    let game = GameState::new(5, data);

    println!("{game:#?}");

    // let p = Player {
    //     name: "your mom".into(),
    //     cash: 5,
    //     assets: vec![Asset {
    //         title: "bank thing".to_string(),
    //         gold_value: 2,
    //         silver_value: 3,
    //         color: Color::Red,
    //         ability: None,
    //         image_front_url: "ablcd".to_string(),
    //         image_back_url: "ablcd".to_string().into(),
    //     }],
    //     liabilities: vec![],
    //     hand: vec![],
    //     cards_drawn: vec![],
    //     assets_to_play: 1,
    //     liabilities_to_play: 1,
    // };

    // println!("{}", serde_json::to_string(&p).unwrap());

    Ok(())
}
