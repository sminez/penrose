use penrose::{core::layout::Layout, extensions::layout::bsp::BSP, util::print_layout_result};
use std::io::{stdin, stdout, Write};

fn main() {
    let mut bsp = BSP::default();
    bsp.split();
    let mut n = 1;

    loop {
        // Clear and reset the screen
        print!("{esc}[2J{esc}[1;1H", esc = 27 as char);
        println!("{} ({n} clients)", bsp.name());
        print_layout_result(&mut bsp, n, 120, 38);

        // TODO: the fact that we need to explicitly split the BSP as well
        //       as adding the client is a problem!
        match read_input().as_str() {
            "+" => {
                n += 1;
                bsp.split();
            }
            //  TODO: need to be able to remove nodes correctly!
            // "-" => {
            //     n = n.saturating_sub(1);
            // }
            "/" | "toggle" => bsp.toggle_orientation(),
            "@" | "rot" | "rotate" => bsp.rotate(),
            ">" | "expand" => bsp.expand_split(),
            "<" | "shrink" => bsp.shrink_split(),
            "u" | "up" => bsp.focus_up(),
            "l" | "left" => bsp.focus_left(),
            "r" | "right" => bsp.focus_right(),
            "*" | "focus_only" => bsp.focused_only = !bsp.focused_only,

            "q" | "quit" => break,
            "?" => {
                println!(">> Current tree:");
                println!("{bsp:#?}");
                read_input();
            }
            _ => println!("unknown input"),
        }
    }
}

fn read_input() -> String {
    stdout().flush().unwrap();
    let mut user_input = String::new();
    stdin().read_line(&mut user_input).unwrap();

    user_input.trim().to_string()
}
