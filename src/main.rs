mod game_events;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _game = game_events::load_game()?;
    println!("Hello, world!");
    Ok(())
}
