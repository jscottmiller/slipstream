#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum System {
    SegaModel2,
}

impl System {
    pub fn label(self) -> &'static str {
        match self {
            System::SegaModel2 => "Sega Model 2",
        }
    }
}

pub struct GameDef {
    pub id: &'static str,
    pub title: &'static str,
    pub year: u16,
    pub manufacturer: &'static str,
    pub system: System,
    /// MAME-style parent ROM set name; `<rom_name>.zip` is looked up in the
    /// user's ROM directory. Slipstream never downloads ROMs.
    pub rom_name: &'static str,
    pub emulator_id: &'static str,
}

pub static GAMES: &[GameDef] = &[
    GameDef {
        id: "daytona",
        title: "Daytona USA",
        year: 1994,
        manufacturer: "Sega AM2",
        system: System::SegaModel2,
        rom_name: "daytona",
        emulator_id: "m2",
    },
    GameDef {
        id: "srallyc",
        title: "Sega Rally Championship",
        year: 1995,
        manufacturer: "Sega AM5",
        system: System::SegaModel2,
        rom_name: "srallyc",
        emulator_id: "m2",
    },
];
