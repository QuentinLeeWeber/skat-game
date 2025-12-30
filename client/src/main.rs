use postcard::{from_bytes, to_stdvec};
use proto::*;
use slint::{Model, ModelRc, VecModel, Weak};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::mpsc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

slint::include_modules!();

const IP_ADDR: &str = "127.0.0.1:6969";

#[derive(Clone, Debug)]
enum AppState {
    Login,
    Ready { name: String },
}

struct AppModel {
    state: AppState,
    hand: Rc<VecModel<Card>>,
}

impl AppModel {
    fn new(ui: &Weak<MainWindow>) -> Self {
        let hand = Rc::new(VecModel::from(Vec::<Card>::new()));
        if let Some(ui) = ui.upgrade() {
            ui.set_hand(ModelRc::from(Rc::clone(&hand)));
        }

        Self {
            state: AppState::Login,
            hand,
        }
    }

    fn name(&self) -> Option<&str> {
        match &self.state {
            AppState::Ready { name } => Some(name),
            _ => None,
        }
    }

    fn has_name(&self) -> bool {
        !matches!(self.state, AppState::Login)
    }

    fn submit_name(&mut self, name: String) {
        if !name.trim().is_empty() {
            self.state = AppState::Ready { name };
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), slint::PlatformError> {
    let ui = MainWindow::new()?;
    let ui_weak = ui.as_weak();

    let app_model = Rc::new(RefCell::new(AppModel::new(&ui_weak)));

    let (sock_tx, sock_rx) = mpsc::channel::<Message>();

    match TcpStream::connect(IP_ADDR).await {
        Ok(tcp_stream) => {
            let (reader, mut writer) = tokio::net::TcpStream::into_split(tcp_stream);
            let reader = BufReader::new(reader);

            tokio::spawn(async move {
                while let Ok(msg) = sock_rx.recv() {
                    println!("sending Messeage: {:?}", msg);
                    let mut msg = to_stdvec(&msg).unwrap();
                    msg.push(b'\n');
                    writer.write_all(&msg).await.unwrap();
                }
            });
        }
        Err(e) => println!("could not connect to server: {}", e),
    };

    ui.on_play_card({
        let app_model = Rc::clone(&app_model);

        move |card| {
            println!("Playing card: {:?}", card);
            let index = app_model.borrow().hand.iter().position(|c| c == card);
            if let Some(i) = index {
                app_model.borrow_mut().hand.remove(i);
            }
        }
    });

    ui.on_set_position({
        let app_model = Rc::clone(&app_model);

        move |from, to| {
            println!("Moving card from {} to {}", from, to);
            let card = app_model.borrow_mut().hand.remove(from as usize);
            app_model.borrow_mut().hand.insert(to as usize, card);
        }
    });

    ui.on_submit_name({
        println!("submut name");
        let app_model = Rc::clone(&app_model);

        move |name| {
            if name == "" {
                return;
            }
            app_model.borrow_mut().submit_name(name.to_string());
            if let Some(ui) = ui_weak.upgrade() {
                let m = app_model.borrow();
                ui.set_has_name(m.has_name());

                let name = m.name().unwrap_or("").to_string();
                let _ = sock_tx.send(Message::Login(name.clone()));
                ui.set_name(name.into());
            }
        }
    });

    let weak_app = ui.as_weak();
    tokio::spawn(async move {
        main_loop(weak_app).await.unwrap();
    });

    ui.run()
}

async fn main_loop(ui: Weak<MainWindow>) -> Result<(), slint::PlatformError> {
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
        let ui = ui.clone();
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(ui) = ui.upgrade() {
                let hand_model = ui.get_hand();
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
