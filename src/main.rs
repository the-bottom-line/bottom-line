mod game;

use game::*;

fn main() {
    let p = Player {
        name: "WHAT".to_string(),
    };

    println!("{}", serde_json::to_string(&p).unwrap());
}
