use std::{env, io::{self, BufRead}};
use std::{thread, time};

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

//https://stackoverflow.com/questions/52906921/zero-cost-abstractions-performance-of-for-loop-vs-iterators
//rust的iter map filter应该也是zero cost abstration
//https://www.youtube.com/watch?v=JCGjjk5ccV0 rust和自动优化filter map sum这些，甚至还会自动用simd去优化
//collect怎么优化？
//https://doc.rust-lang.org/std/iter/trait.Iterator.html
fn watch_softirqs() {
    let mut counts = [0u64;128];
    let mut first = true;
    let mut times = 0;
    loop {
        let file = std::fs::File::open("/proc/softirqs");
        let file = match file {
            Ok(f) => f,
            Err(e) => {
                println!("open err:{:?}", e);
                return;
            }
        };

        let line = io::BufReader::new(file).lines()
            .filter(|line| line.as_ref().map_or_else(|_err| false, |line| line.contains("NET_RX")))
            .nth(0)
            .unwrap_or(Err(std::io::Error::new(std::io::ErrorKind::Other, "foo")))
            .map_or("".to_string(), |line| line);

        line.split_ascii_whitespace().collect::<Vec<&str>>()
            .iter()
            .enumerate()
            .for_each(|(i, item)| {
                if i == 0 {
                    return;
                }
                let v =  item.parse::<u64>().unwrap();
                //println!("{}", item);
                if !first {
                    println!("{} CPU{} times{}",v - counts[i], i-1, times);
                }
                counts[i] = v;
            });

        first = false;
        times += 1;
        thread::sleep(time::Duration::from_secs(1));
    }
}