use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    println!("{:?}", args);

    let cmd = args.get(1);

    if let Some(cmd) = cmd {
        match &cmd[..] {
        "softirqs" => watch_softirqs(),
        _ => println!("other command"),

        }
    } else {
        panic!("Unknown command")
    }
}

fn watch_softirqs() {
    println!("xx")
}