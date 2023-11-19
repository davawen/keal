pub struct Arguments {
    pub dmenu: bool,
    pub protocol: Protocol
}

#[derive(Clone, Copy)]
pub enum Protocol {
    RofiExtended,
    Keal
}

pub enum Error {
    Exit,
    UnknownFlag(String)
}

impl Arguments {
    pub fn parse() -> Result<Self, Error> {
        let mut arguments = Arguments {
            dmenu: false,
            protocol: Protocol::RofiExtended
        };

        let mut args = std::env::args();
        let _ = args.next(); // ignore executable name
        for arg in args {
            match arg.as_str() {
                "--dmenu" | "-d" => arguments.dmenu = true,
                "--keal" | "-k" => arguments.protocol = Protocol::Keal,
                "--help" | "-h" => {
                    Self::print_help();
                    Err(Error::Exit)?
                }
                "--version" | "-v" => {
                    Self::print_version();
                    Err(Error::Exit)?
                }
                _ => Err(Error::UnknownFlag(arg))?
            }
        }

        Ok(arguments)
    }

    fn print_version() {
        println!("keal: version {}", env!("CARGO_PKG_VERSION"));
    }

    fn print_help() {
        println!("usage: keal [options...]");
        println!();
        println!("options:");
        println!("  -h, --help    Show this help and exit");
        println!("  -v, --version Show the current version of keal");
        println!("  -d, --dmenu   Launch keal in dmenu mode (pipe choices into it)");
        println!("  -k, --keal    In dmenu mode, use the same protocol as plugins, instead of the default rofi extended dmenu protocol");
    }
}
