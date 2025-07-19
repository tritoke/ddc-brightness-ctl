use ddc::{Ddc, DdcHost as _};
use ddc_hi::Display;
use std::{ops::Neg, process::ExitCode};

const RED: &str = "\x1B[31m";
const RESET: &str = "\x1B[0m";

const LUMINANCE_FEATURE_CODE: u8 = 0x10;

struct Args {
    action: Action,
    display: Option<usize>,
    list: bool,
}

#[derive(Clone, Copy)]
enum Action {
    Change(BrightnessChange),
    Get,
}

impl Action {
    fn is_noop(self) -> bool {
        matches!(self, Action::Change(BrightnessChange::Relative(0)))
    }

    fn execute(self, display: &mut Display, display_no: usize) -> ExitCode {
        let mut exit_code = ExitCode::SUCCESS;

        let model = display
            .info
            .model_name
            .as_deref()
            .unwrap_or("Unknown Model");

        let disp = format!("display {display_no} ({model})");

        let Ok(vcp) = display.handle.get_vcp_feature(LUMINANCE_FEATURE_CODE) else {
            eprintln!("{RED}Timed out waiting for response from {disp}{RESET}");
            return ExitCode::FAILURE;
        };
        let old_value = vcp.value();
        display.handle.sleep();

        match self {
            Action::Change(brightness_change) => {
                let new_value = brightness_change.apply(old_value);
                if old_value == new_value {
                    println!("No change needed for {disp}");
                    return ExitCode::SUCCESS;
                }

                println!("Changing brighness of {disp} from {old_value} to {new_value}");
                if let Err(e) = display
                    .handle
                    .set_vcp_feature(LUMINANCE_FEATURE_CODE, new_value)
                {
                    eprintln!("{RED}Failed to set brightness for {disp}: {e}{RESET}");
                    exit_code = ExitCode::FAILURE;
                }
                display.handle.sleep();
            }
            Action::Get => {
                println!("{disp} is set to {old_value}% brightness");
            }
        }

        exit_code
    }
}

#[derive(Clone, Copy)]
enum BrightnessChange {
    Relative(i16),
    Absolute(u16),
}

impl BrightnessChange {
    fn apply(self, value: u16) -> u16 {
        match self {
            Self::Relative(offset) => {
                let default = if offset < 0 { 0 } else { 100 };
                value.checked_add_signed(offset).unwrap_or(default)
            }
            Self::Absolute(value) => value,
        }
        .clamp(0, 100)
    }
}

fn parse_args() -> Result<Args, lexopt::Error> {
    use lexopt::prelude::*;

    let mut parser = lexopt::Parser::from_env();
    let mut display = None;
    let mut action = Action::Get;
    let mut list = false;
    while let Some(arg) = parser.next()? {
        match arg {
            Short('d') | Long("display") => {
                display = Some(parser.value()?.parse()?);
            }
            Long("inc") => {
                action = Action::Change(BrightnessChange::Relative(parser.value()?.parse()?));
            }
            Long("dec") => {
                action = Action::Change(BrightnessChange::Relative(
                    parser.value()?.parse::<i16>()?.neg(),
                ));
            }
            Long("set") => {
                action = Action::Change(BrightnessChange::Absolute(parser.value()?.parse()?))
            }
            Long("get") => action = Action::Get,
            Short('l') | Long("list") => list = true,
            Short('v') | Long("version") => {
                println!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            Short('h') | Long("help") => {
                println!("Usage: ddc-brightness-ctl [-h|--help] [-v|--version] [-d|--display=NUM] [-l|--list] [--inc=NUM] [--dec=NUM] [--set=NUM]");
                println!();
                println!("Options:");
                println!("  -d,    --display: optionally specify which display to change");
                println!("                    default operates on all displays");
                println!("  -l,       --list: list all detected displays and metadata");
                println!("  -v,    --version: get the program version");
                println!("  -h,       --help: print this help message");
                println!("             --get: get the current brightness");
                println!("             --set: set brightness to NUM percent");
                println!("             --inc: increase brightness by NUM percent");
                println!("             --dec: decrease brightness by NUM percent");
                std::process::exit(0);
            }
            _ => return Err(arg.unexpected()),
        }
    }

    Ok(Args {
        action,
        display,
        list,
    })
}

fn main() -> ExitCode {
    let Args {
        action,
        display,
        list,
    } = match parse_args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{RED}Failed to parse arguments: {e}{RESET}");
            return ExitCode::FAILURE;
        }
    };

    if action.is_noop() && !list {
        return ExitCode::SUCCESS;
    }

    println!("Querying display info... (~1-2 seconds)");
    let mut displays = Display::enumerate();

    if list {
        println!("Detected displays:");
        for (i, disp) in displays.iter().enumerate() {
            println!(
                "  - [{i}]: {} - ({}:{}:{}), manufactured week {} of {}",
                disp.info.model_name.as_deref().unwrap_or("Unknown Model"),
                disp.info.manufacturer_id.as_deref().unwrap_or("???"),
                disp.info
                    .model_id
                    .map(|num| format!("{num:04X}"))
                    .as_deref()
                    .unwrap_or("????"),
                disp.info
                    .serial
                    .map(|num| format!("{num:08X}"))
                    .as_deref()
                    .unwrap_or("????????"),
                disp.info
                    .manufacture_week
                    .map(|num| format!("{num}"))
                    .as_deref()
                    .unwrap_or("??"),
                disp.info
                    .manufacture_year
                    .map(|num| format!("{}", 1990 + num as u16))
                    .as_deref()
                    .unwrap_or("????"),
            );
        }

        return ExitCode::SUCCESS;
    }

    if let Some(n) = display {
        if let Some(disp) = displays.get_mut(n) {
            return action.execute(disp, n);
        } else {
            eprintln!("{RED}No display {n}{RESET}");
            return ExitCode::FAILURE;
        }
    }

    let mut exit_code = ExitCode::SUCCESS;
    for (i, mut disp) in displays.into_iter().enumerate() {
        if action.execute(&mut disp, i) == ExitCode::FAILURE {
            exit_code = ExitCode::FAILURE;
        }
    }

    exit_code
}
