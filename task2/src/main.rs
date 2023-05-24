pub mod input;
pub use input::*;
pub mod node;
pub use node::*;

fn main() {
    println!("Starting system simulation...");

    if let Some(filename) = std::env::args().nth(1) {
        let file = std::fs::File::open(&filename).expect("failed to open the file");
        println!("Reading file: {filename}");

        let instructions: input::Task2StudyCaseInstructions =
            serde_json::from_reader(file).unwrap();

        let mut runners = Vec::<NodeRunner>::new();

        for (node_name, node_instructions) in instructions.0 {
            let nr = NodeRunner::new(Node::new(node_name, node_instructions.0));
            for other in &runners {
                let own = nr.give_registration_data();
                other.register_new_connection(own.0, own.1);
                let other = other.give_registration_data();
                nr.register_new_connection(other.0, other.1);
            }
            runners.push(nr);
        }
        for nr in runners {
            nr.start();
        }
    } else {
        println!("No filename provided as an input, proceeding to run a simulation with 10 nodes and rondomized task durations");
        let mut runners = Vec::<NodeRunner>::new();

        for _ in 0..9 {
            let nr = NodeRunner::new(Node::default());
            for other in &runners {
                let own = nr.give_registration_data();
                other.register_new_connection(own.0, own.1);
                let other = other.give_registration_data();
                nr.register_new_connection(other.0, other.1);
            }
            runners.push(nr);
        }
        for nr in runners {
            nr.start();
        }
    }

    // make sure we do not exit too early
    loop {}
}
