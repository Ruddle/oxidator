use actix::prelude::*;
use std::time::Duration;

#[derive(Message)]
struct Ping {
    pub id: usize,
}

// Actor definition
struct Game {
    name: String,
    counter: usize,
    addr: Recipient<Ping>,
}

impl Actor for Game {
    type Context = Context<Game>;
}

// simple message handler for Ping message
impl Handler<Ping> for Game {
    type Result = ();

    fn handle(&mut self, msg: Ping, ctx: &mut Context<Self>) {
        self.counter += 1;

        if self.counter > 10 {
            System::current().stop();
        } else {
            println!("Ping {} received {:?}", self.name, msg.id);

            self.addr.do_send(Ping { id: msg.id + 1 });
            // wait 100 nanos
            //            ctx.run_later(Duration::new(0, 100), move |act, _| {
            //                act.addr.do_send(Ping { id: msg.id + 1 });
            //            });
        }
    }
}

pub fn main() {
    let system = System::new("test");

    // To get a Recipient object, we need to use a different builder method
    // which will allow postponing actor creation
    let addr = Game::create(|ctx| {
        // now we can get an address of the first actor and create the second actor
        let addr = ctx.address();

        let reci = addr.recipient();

        let addr2 = Game {
            name: "2".to_string(),
            counter: 0,
            addr: reci.clone(),
        }
        .start();

        // let's start pings
        addr2.do_send(Ping { id: 10 });

        // now we can finally create first actor
        Game {
            name: "1".to_string(),
            counter: 0,
            addr: reci.clone(),
        }
    });

    system.run();
}
