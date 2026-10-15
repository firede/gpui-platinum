//! The gray ramp and six accents of the Platinum desktop.

pub const WHITE: u32 = 0xffffff;
pub const GRAY_1: u32 = 0xeeeeee;
pub const GRAY_2: u32 = 0xdddddd;
pub const GRAY_3: u32 = 0xcccccc;
pub const GRAY_4: u32 = 0xbbbbbb;
pub const GRAY_5: u32 = 0xaaaaaa;
pub const GRAY_6: u32 = 0x999999;
pub const GRAY_7: u32 = 0x888888;
pub const GRAY_8: u32 = 0x777777;
pub const GRAY_9: u32 = 0x666666;
pub const GRAY_10: u32 = 0x555555;
pub const GRAY_11: u32 = 0x444444;
pub const GRAY_12: u32 = 0x222222;
pub const BLACK: u32 = 0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Accent {
    pub name: &'static str,
    pub dark: u32,
    pub mid: u32,
    pub light: u32,
}

pub const ACCENTS: [Accent; 6] = [
    Accent {
        name: "Lavender",
        dark: 0x4c4ca8,
        mid: 0x8888dd,
        light: 0xccccff,
    },
    Accent {
        name: "Graphite",
        dark: 0x4a4a4a,
        mid: 0x8c8c8c,
        light: 0xcccccc,
    },
    Accent {
        name: "Harbor",
        dark: 0x1f5a8c,
        mid: 0x5c99cc,
        light: 0xbbddff,
    },
    Accent {
        name: "Moss",
        dark: 0x3a6b2a,
        mid: 0x77aa5e,
        light: 0xcceebb,
    },
    Accent {
        name: "Ember",
        dark: 0x993d1f,
        mid: 0xdd8855,
        light: 0xffddbb,
    },
    Accent {
        name: "Orchid",
        dark: 0x7a2e7a,
        mid: 0xbb77bb,
        light: 0xeeccee,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    pub accent: Accent,
    pub background: u32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            accent: ACCENTS[0],
            background: GRAY_2,
        }
    }
}
