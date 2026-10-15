//! Bitmap titles used by the desktop application menus.

const MINI_DESK: &[&str] = &[
    "............",
    "####........",
    "#oo#######..",
    "#oooooooo#..",
    "#o++++++o#..",
    "#o++++++o#..",
    "#o++++++o#..",
    "#o++++++o#..",
    "#oooooooo#..",
    "##########..",
    "............",
    "............",
];

const MINI_CONTROLS: &[&str] = &[
    "............",
    ".##########.",
    ".#wwwwwwww#.",
    ".#w##w:::w#.",
    ".#w##w:::w#.",
    ".#wwwwwwww#.",
    ".#w::::::w#.",
    ".#wooooo:w#.",
    ".#wwwwwwww#.",
    ".##########.",
    "............",
    "............",
];

const MINI_CALC: &[&str] = &[
    ".##########.",
    ".#::::::::#.",
    ".#:wwwwww:#.",
    ".#::::::::#.",
    ".#:w:w:w::#.",
    ".#::::::::#.",
    ".#:w:w:w::#.",
    ".#::::::::#.",
    ".#:w:w:w::#.",
    ".#::::::::#.",
    ".##########.",
    "............",
];

const MINI_NOTE: &[&str] = &[
    "..#.#.#.#...",
    ".#########..",
    ".#wwwwwww#..",
    ".#w:::::w#..",
    ".#wwwwwww#..",
    ".#w:::::w#..",
    ".#wwwwwww#..",
    ".#w:::::w#..",
    ".#wwwwwww#..",
    ".#########..",
    "............",
    "............",
];

const MINI_PAGE: &[&str] = &[
    ".#######....",
    ".#wwwww##...",
    ".#wwwww#w#..",
    ".#w:::w####.",
    ".#wwwwwwww#.",
    ".#w::::::w#.",
    ".#wwwwwwww#.",
    ".#w::::::w#.",
    ".#wwwwwwww#.",
    ".##########.",
    "............",
    "............",
];

const MINI_GRID: &[&str] = &[
    "############",
    "#oo#ww#ww#w#",
    "############",
    "#ww#ww#ww#w#",
    "############",
    "#ww#ww#ww#w#",
    "############",
    "#ww#ww#ww#w#",
    "############",
    "............",
    "............",
    "............",
];

const MINI_TILES: &[&str] = &[
    "#####.#####.",
    "#+++#.#ooo#.",
    "#+++#.#ooo#.",
    "#####.#####.",
    "............",
    "#####.#####.",
    "#ooo#.#:::#.",
    "#ooo#.#:::#.",
    "#####.#####.",
    "............",
    "............",
    "............",
];

const MINI_CLOCK: &[&str] = &[
    "...######...",
    "..#wwwwww#..",
    ".#www#www#..",
    "#wwww#wwww#.",
    "#wwww#wwww#.",
    "#wwww###ww#.",
    "#wwwwwwwww#.",
    "#wwwwwwwww#.",
    ".#wwwwwww#..",
    "..#wwwww#...",
    "...#####....",
    "............",
];

const MINI_PANEL: &[&str] = &[
    "##########..",
    "#::::::::#..",
    "#:######:#..",
    "#:#oooo#:#..",
    "#:#oooo#:#..",
    "#:######:#..",
    "#::::::::#..",
    "##########..",
    "...####.....",
    "..######....",
    "............",
    "............",
];

pub const DESK_LOGO: &[&str] = &[
    "....##....",
    "...#oo#...",
    "..#oooo#..",
    ".#oo++oo#.",
    "#oo+ww+oo#",
    "#oo+ww+oo#",
    ".#oo++oo#.",
    "..#oooo#..",
    "...#oo#...",
    "....##....",
];

pub fn application_icon(key: &str) -> &'static [&'static str] {
    match key {
        "controls" => MINI_CONTROLS,
        "calculator" => MINI_CALC,
        "jotter" => MINI_NOTE,
        "scribe" => MINI_PAGE,
        "ledger" => MINI_GRID,
        "puzzle" => MINI_TILES,
        "alarmClock" => MINI_CLOCK,
        "appearance" | "dateTime" | "sound" | "mouse" | "language" => MINI_PANEL,
        _ => MINI_DESK,
    }
}
