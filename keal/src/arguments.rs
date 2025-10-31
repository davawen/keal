use std::sync::OnceLock;

pub struct Arguments {
    pub dmenu: bool,
    pub protocol: Protocol,
    pub timings: bool,
    pub frontend: &'static str
}

#[derive(Clone, Copy)]
pub enum Protocol {
    RofiExtended,
    Keal
}

static ARGUMENTS: OnceLock<Arguments> = OnceLock::new();
pub fn arguments() -> &'static Arguments {
    ARGUMENTS.get().expect("arguments should have been initialized in main")
}

pub enum Error {
    Exit,
    UnknownFlag(String)
}

pub trait Frontend {
    const FRONTEND_NAME: &'static str;
}

impl Arguments {
    pub fn init(frontend: &'static str) -> Result<&'static Self, Error> {
        let this = Self::parse(frontend)?;
        let arguments = ARGUMENTS.get_or_init(move || this);
        Ok(arguments)
    }

    fn parse(frontend: &'static str) -> Result<Self, Error> {
        let mut arguments = Arguments {
            dmenu: false,
            protocol: Protocol::RofiExtended,
            timings: false,
            frontend
        };

        let mut args = std::env::args();
        let _ = args.next(); // ignore executable name
        for arg in args {
            match arg.as_str() {
                "--dmenu" | "-d" => arguments.dmenu = true,
                "--keal" | "-k" => arguments.protocol = Protocol::Keal,
                "--timings" => arguments.timings = true,
                "--help" | "-h" => {
                    Self::print_help();
                    Err(Error::Exit)?
                }
                "--version" | "-v" => {
                    Self::print_version(frontend);
                    Err(Error::Exit)?
                }
                _ => Err(Error::UnknownFlag(arg))?
            }
        }

        Ok(arguments)
    }

    fn print_version(frontend: &'static str) {
        println!("keal version {}, {frontend} frontend", env!("CARGO_PKG_VERSION"));
    }

    fn print_help() {
        println!("usage: keal [options...]");
        println!();
        println!("options:");
        println!("  -h, --help    Show this help and exit");
        println!("  -v, --version Show the current version and frontend of keal");
        println!("  -d, --dmenu   Launch keal in dmenu mode (pipe choices into it)");
        println!("  -k, --keal    In dmenu mode, use the same protocol as plugins, instead of the default rofi extended dmenu protocol");
        println!("      --timings Show how long the different keal systems take to start up")
    }
}
