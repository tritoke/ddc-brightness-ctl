use ddc::{Ddc, DdcHost as _};
use ddc_hi::Display;
use std::{ops::Neg, process::ExitCode};

const RED: &str = "\x1B[31m";
const RESET: &str = "\x1B[0m";

struct Args {
    brightness_change: BrightnessChange,
    display: Option<usize>,
    list: bool,
}

#[derive(Clone, Copy)]
enum BrightnessChange {
    Relative(i16),
    Absolute(u16),
}

impl BrightnessChange {
    fn is_noop(self) -> bool {
        matches!(self, BrightnessChange::Relative(0))
    }

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
    let mut brightness_change = BrightnessChange::Relative(0);
    let mut list = false;
    while let Some(arg) = parser.next()? {
        match arg {
            Short('d') | Long("display") => {
                display = Some(parser.value()?.parse()?);
            }
            Long("inc") => {
                brightness_change = BrightnessChange::Relative(parser.value()?.parse()?);
            }
            Long("dec") => {
                brightness_change =
                    BrightnessChange::Relative(parser.value()?.parse::<i16>()?.neg());
            }
            Long("set") => {
                brightness_change = BrightnessChange::Absolute(parser.value()?.parse()?);
            }
            Short('l') | Long("list") => {
                list = true;
            }
            Short('h') | Long("help") => {
                println!("Usage: ddc-brightness-ctl [-d|--display=NUM] [-l|--list] [--inc=NUM] [--dec=NUM] [--set=NUM]");
                println!();
                println!("Options:");
                println!("  -d, --display: optionally specify which display to change");
                println!("                 default operates on all displays");
                println!("  -l,    --list: list all detected displays and metadata");
                println!("          --set: set brightness to NUM percent");
                println!("          --inc: increase brightness by NUM percent");
                println!("          --dec: decrease brightness by NUM percent");
                std::process::exit(0);
            }
            _ => return Err(arg.unexpected()),
        }
    }

    Ok(Args {
        brightness_change,
        display,
        list,
    })
}

fn main() -> ExitCode {
    let Args {
        brightness_change,
        display,
        list,
    } = match parse_args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{RED}Failed to parse arguments: {e}{RESET}");
            return ExitCode::FAILURE;
        }
    };

    if brightness_change.is_noop() && !list {
        return ExitCode::SUCCESS;
    }

    println!("Querying display info... (~1-2 seconds)");
    let mut displays = Display::enumerate();

    if list {
        for (i, disp) in displays.iter().enumerate() {
            println!("  - [{i}]: {:?}", disp.info);
        }
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
            change_brightness(disp, n, brightness_change);
        }
        return ExitCode::SUCCESS;
    }

    for (i, mut disp) in displays.into_iter().enumerate() {
        change_brightness(&mut disp, i, brightness_change);
    }

    ExitCode::SUCCESS
}

fn change_brightness(display: &mut Display, display_no: usize, change: BrightnessChange) {
    let model = display
        .info
        .model_name
        .as_deref()
        .unwrap_or("Unknown Model");

    let disp = format!("{display_no} ({model})");

    let luminance_feature_code = 0x10;
    let Ok(vcp) = display.handle.get_vcp_feature(luminance_feature_code) else {
        eprintln!("{RED}Timed out waiting for response from display {disp}{RESET}");
        return;
    };
    display.handle.sleep();

    let old_value = vcp.value();
    let new_value = change.apply(old_value);
    if old_value == new_value {
        println!("No change needed for {disp}");
        return;
    }

    println!("Changing brighness of {disp} from {old_value} to {new_value}");
    if let Err(e) = display
        .handle
        .set_vcp_feature(luminance_feature_code, new_value)
    {
        eprintln!("{RED}Failed to set brightness for display {disp}: {e}{RESET}");
    }

    display.handle.sleep();
}
