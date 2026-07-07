#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum System {
    SegaModel2,
    SegaModel3,
}

impl System {
    pub fn label(self) -> &'static str {
        match self {
            System::SegaModel2 => "Sega Model 2",
            System::SegaModel3 => "Sega Model 3",
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
    GameDef {
        id: "scud",
        title: "Scud Race",
        year: 1996,
        manufacturer: "Sega AM2",
        system: System::SegaModel3,
        rom_name: "scud",
        emulator_id: "supermodel",
    },
    GameDef {
        id: "lemans24",
        title: "Le Mans 24",
        year: 1997,
        manufacturer: "Sega AM4",
        system: System::SegaModel3,
        rom_name: "lemans24",
        emulator_id: "supermodel",
    },
    GameDef {
        id: "daytona2",
        title: "Daytona USA 2: Battle on the Edge",
        year: 1998,
        manufacturer: "Sega AM2",
        system: System::SegaModel3,
        rom_name: "daytona2",
        emulator_id: "supermodel",
    },
    GameDef {
        id: "dayto2pe",
        title: "Daytona USA 2: Power Edition",
        year: 1998,
        manufacturer: "Sega AM2",
        system: System::SegaModel3,
        rom_name: "dayto2pe",
        emulator_id: "supermodel",
    },
    GameDef {
        id: "srally2",
        title: "Sega Rally 2",
        year: 1998,
        manufacturer: "Sega AM5",
        system: System::SegaModel3,
        rom_name: "srally2",
        emulator_id: "supermodel",
    },
];
