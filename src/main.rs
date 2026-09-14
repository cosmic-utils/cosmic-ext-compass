// SPDX-License-Identifier: GPL-3.0-only

use clap::Parser;
use compass::app::{CompassApp, Flags};
use i18n_embed::DesktopLanguageRequester;

#[derive(Debug, Parser)]
#[command(
    name = "compass",
    about = "A responsive compass for Linux",
    version = env!("GIT_VERSION")
)]
struct Cli {
    /// Animate smooth, natural phone rotation without using SensorProxy.
    #[arg(long, conflicts_with = "demo_heading")]
    demo: bool,

    /// Show a fixed heading instead of connecting to SensorProxy.
    #[arg(long, value_name = "DEGREES", value_parser = parse_demo_heading)]
    demo_heading: Option<f32>,
}

fn parse_demo_heading(value: &str) -> Result<f32, String> {
    let degrees: f32 = value
        .parse()
        .map_err(|error| format!("invalid heading: {error}"))?;
    if !degrees.is_finite() {
        return Err("heading must be finite".to_owned());
    }
    Ok(degrees.rem_euclid(360.0))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("compass=info")),
        )
        .init();

    let cli = Cli::parse();
    compass::i18n::init(&DesktopLanguageRequester::requested_languages());

    let settings = cosmic::app::Settings::default()
        .size(cosmic::iced::Size::new(360.0, 640.0))
        .size_limits(
            cosmic::iced::Limits::NONE
                .min_width(240.0)
                .min_height(240.0),
        );
    cosmic::app::run::<CompassApp>(
        settings,
        Flags {
            demo_heading: cli.demo_heading,
            demo_motion: cli.demo,
        },
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn demo_flag_selects_animated_motion() {
        let cli = Cli::try_parse_from(["compass", "--demo"]).expect("--demo should parse");

        assert!(cli.demo);
        assert_eq!(cli.demo_heading, None);
    }

    #[test]
    fn cli_uses_the_build_version() {
        assert_eq!(Cli::command().get_version(), Some(env!("GIT_VERSION")));
    }
}
