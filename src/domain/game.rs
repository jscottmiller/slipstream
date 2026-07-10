#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum System {
    SegaModel2,
    SegaModel3,
}

/// What a game is played with; drives emulator input config and how the
/// cabinet UI groups its rail.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ControlKind {
    Wheel,
    Lightgun,
}

impl ControlKind {
    pub fn label(self) -> &'static str {
        match self {
            ControlKind::Wheel => "Wheel",
            ControlKind::Lightgun => "Lightgun",
        }
    }
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
    pub controls: ControlKind,
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
        controls: ControlKind::Wheel,
    },
    GameDef {
        id: "srallyc",
        title: "Sega Rally Championship",
        year: 1995,
        manufacturer: "Sega AM5",
        system: System::SegaModel2,
        rom_name: "srallyc",
        emulator_id: "m2",
        controls: ControlKind::Wheel,
    },
    GameDef {
        id: "scud",
        title: "Scud Race",
        year: 1996,
        manufacturer: "Sega AM2",
        system: System::SegaModel3,
        rom_name: "scud",
        emulator_id: "supermodel",
        controls: ControlKind::Wheel,
    },
    GameDef {
        id: "lemans24",
        title: "Le Mans 24",
        year: 1997,
        manufacturer: "Sega AM4",
        system: System::SegaModel3,
        rom_name: "lemans24",
        emulator_id: "supermodel",
        controls: ControlKind::Wheel,
    },
    GameDef {
        id: "daytona2",
        title: "Daytona USA 2: Battle on the Edge",
        year: 1998,
        manufacturer: "Sega AM2",
        system: System::SegaModel3,
        rom_name: "daytona2",
        emulator_id: "supermodel",
        controls: ControlKind::Wheel,
    },
    GameDef {
        id: "dayto2pe",
        title: "Daytona USA 2: Power Edition",
        year: 1998,
        manufacturer: "Sega AM2",
        system: System::SegaModel3,
        rom_name: "dayto2pe",
        emulator_id: "supermodel",
        controls: ControlKind::Wheel,
    },
    GameDef {
        id: "srally2",
        title: "Sega Rally 2",
        year: 1998,
        manufacturer: "Sega AM5",
        system: System::SegaModel3,
        rom_name: "srally2",
        emulator_id: "supermodel",
        controls: ControlKind::Wheel,
    },
    GameDef {
        id: "vcop",
        title: "Virtua Cop",
        year: 1994,
        manufacturer: "Sega AM2",
        system: System::SegaModel2,
        rom_name: "vcop",
        emulator_id: "m2",
        controls: ControlKind::Lightgun,
    },
    GameDef {
        id: "vcop2",
        title: "Virtua Cop 2",
        year: 1995,
        manufacturer: "Sega AM2",
        system: System::SegaModel2,
        rom_name: "vcop2",
        emulator_id: "m2",
        controls: ControlKind::Lightgun,
    },
    GameDef {
        id: "hotd",
        title: "The House of the Dead",
        year: 1996,
        manufacturer: "Sega AM1",
        system: System::SegaModel2,
        rom_name: "hotd",
        emulator_id: "m2",
        controls: ControlKind::Lightgun,
    },
    GameDef {
        id: "lostwsga",
        title: "The Lost World: Jurassic Park",
        year: 1997,
        manufacturer: "Sega",
        system: System::SegaModel3,
        rom_name: "lostwsga",
        emulator_id: "supermodel",
        controls: ControlKind::Lightgun,
    },
];
