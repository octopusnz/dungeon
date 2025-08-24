use dialoguer::{Select, theme::ColorfulTheme};
use rand::Rng;

fn main() {
    let options = vec![
        "PickPocket",
        "Other option",
        "Exit program",
    ];

    loop {
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Choose an option")
            .items(&options)
            .default(0)
            .interact()
            .unwrap();

        match selection {
            0 => pick_pocket(),
            1 => other_option(),
            2 => {
                println!("Exiting");
                break;
            },
            _ => unreachable!(), // dialoguer ensures only valid indices
        }
    }
}

fn pick_pocket() {
    let mut rng = rand::rng();
    let success = rng.random_bool(0.5);
    if success {
        println!("Success! You stole 15 gold coins 💰");
    } else {
        println!("Caught! The NPC calls the guards 🚨");
    }
}

fn other_option() {
    println!("Thanks for choosing the other option");
}


