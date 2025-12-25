use slint::{Model, ModelRc, VecModel, Weak};
use std::rc::Rc;

slint::include_modules!();

#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {
    let app = MainWindow::new()?;

    let hand_model = Rc::new(VecModel::from(Vec::<Card>::new()));
    app.set_hand(ModelRc::from(Rc::clone(&hand_model)));

    let hm = Rc::clone(&hand_model);
    app.on_play_card(move |card| {
        println!("Playing card: {:?}", card);
        let index = hm.iter().position(|c| c == card);
        if let Some(i) = index {
            hm.remove(i);
        }
    });

    let hm = Rc::clone(&hand_model);
    app.on_set_position(move |from, to| {
        println!("Moving card from {} to {}", from, to);
        let card = hm.remove(from as usize);
        hm.insert(to as usize, card);
    });

    let weak_app = app.as_weak();
    tokio::spawn(async move {
        main_loop(weak_app).await.unwrap();
    });

    app.run()
}

async fn main_loop(app: Weak<MainWindow>) -> Result<(), slint::PlatformError> {
    let cards_to_add = vec![
        Card {
            suit: CardSuit::Heart,
            rank: CardRank::Seven,
        },
        Card {
            suit: CardSuit::Diamond,
            rank: CardRank::Eight,
        },
        Card {
            suit: CardSuit::Clubs,
            rank: CardRank::Nine,
        },
        Card {
            suit: CardSuit::Spade,
            rank: CardRank::Ace,
        },
    ];

    for card in cards_to_add {
        let app = app.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(app) = app.upgrade() {
                let hand_model = app.get_hand();
                let vec_model = hand_model
                    .as_any()
                    .downcast_ref::<VecModel<Card>>()
                    .unwrap();

                vec_model.push(card);
            }
        });
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    Ok(())
}
