//! Tests that layouts behave as exepected
use penrose::{
    builtin::layout::{CenteredMain, Grid, MainAndStack, Monocle},
    core::layout::Layout,
    extensions::layout::{Fibonacci, Tatami},
    pure::{geometry::Rect, Stack},
};
use simple_test_case::dir_cases;
use simple_txtar::{Archive, Builder};
use std::fs;

const R_SCREEN: Rect = Rect::new(0, 0, 1920, 1200);
const MAX_CLIENTS: usize = 10;
const LAYOUTS: [&str; 6] = [
    "MainAndStack",
    "CenteredMain",
    "Grid",
    "Monocle",
    "Fibonacci",
    "Tatami",
];

fn get_layout(name: &str) -> Box<dyn Layout> {
    match name {
        "MainAndStack" => MainAndStack::boxed_default(),
        "CenteredMain" => CenteredMain::boxed_default(),
        "Grid" => Grid::boxed(),
        "Monocle" => Monocle::boxed(),
        "Fibonacci" => Fibonacci::boxed_default(),
        "Tatami" => Tatami::boxed_default(),
        name => panic!("{name} is not a known layout"),
    }
}

fn stringified_positions(layout: &str, n: usize) -> String {
    let mut l = get_layout(layout);
    let mut s = String::new();

    let stack = Stack::try_from_iter(0..n as u32).unwrap().map(Into::into);
    let (_, positions) = l.layout(&stack, R_SCREEN);
    for p in positions {
        s.push_str(&format!("{p:?}\n"));
    }

    s
}

#[test]
#[ignore = "un-ignore to update test data"]
fn update_snapshot_data() {
    for layout in LAYOUTS {
        let mut archive = Builder::new();
        archive.file(("layout", layout));

        for n in 1..=MAX_CLIENTS {
            let expected = stringified_positions(layout, n);
            archive.file((n.to_string(), expected));
        }

        fs::write(
            format!("tests/data/layout/snapshots/{layout}"),
            archive.build().to_string(),
        )
        .unwrap();
    }
}

#[dir_cases("tests/data/layout/snapshots")]
#[test]
fn snapshot(_: &str, content: &str) {
    let a = Archive::from(content);
    let name = a.get("layout").unwrap().content.trim();

    for n in 1..=MAX_CLIENTS {
        let expected = &a.get(&n.to_string()).unwrap().content;
        let computed = stringified_positions(name, n);

        pretty_assertions::assert_eq!(&computed, expected, "n clients = {n}");
    }
}
