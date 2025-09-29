mod game;

use game::*;

fn main() {
    let p = Player {
        name: "your mom".into(),
        cash: 5,
        assets: vec![Asset {
            name: "bank thing".into(),
            gold_value: 2,
            silver_value: 3,
            color: Color::Red,
            asset_powerup: None,
        }],
        liabilities: vec![],
        hand: vec![],
        gold: 2,
        silver: 3,
        cards_to_grab: 3,
        assets_to_play: 1,
        liabilities_to_play: 1,
    };

    println!("{}", serde_json::to_string(&p).unwrap());
}
