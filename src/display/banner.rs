use colored::Colorize;

const LOGO: &str = r#"
 ██████╗  █████╗ ████████╗ █████╗ ██╗   ██╗███████╗██████╗ ███████╗███████╗
 ██╔══██╗██╔══██╗╚══██╔══╝██╔══██╗██║   ██║██╔════╝██╔══██╗██╔════╝██╔════╝
 ██║  ██║███████║   ██║   ███████║██║   ██║█████╗  ██████╔╝███████╗█████╗
 ██║  ██║██╔══██║   ██║   ██╔══██║╚██╗ ██╔╝██╔══╝  ██╔══██╗╚════██║██╔══╝
 ██████╔╝██║  ██║   ██║   ██║  ██║ ╚████╔╝ ███████╗██║  ██║███████║███████╗
 ╚═════╝ ╚═╝  ╚═╝   ╚═╝   ╚═╝  ╚═╝  ╚═══╝  ╚══════╝╚═╝  ╚═╝╚══════╝╚══════╝"#;

const WELCOME_LINES: &[&str] = &[
    "",
    "  τ Bittensor SN13 Data Universe",
    "  ────────────────────────────────────────────────────────────────",
    "  Social data from X/Twitter & Reddit via the",
    "  decentralized Bittensor miner network.",
    "",
    "  Getting started:",
    "    dv auth                          Set up API key",
    "    dv search x -k bitcoin -l 10     Search X posts",
    "    dv search reddit -k r/crypto     Search Reddit",
    "    dv gravity create -p x -t '#btc' Start collection",
    "",
    "  Output modes:",
    "    dv -o json search ...            JSON (for agents)",
    "    dv -o csv search ...             CSV (for Excel)",
    "    dv --dry-run search ...          Preview request",
    "",
    "  More info:",
    "    dv <command> --help              Command help",
    "    dv commands                      Agent JSON catalog",
    "    https://app.macrocosmos.ai       Get API key",
    "",
];

const BOX_WIDTH: usize = 68;

pub fn print_banner() {
    eprintln!();
    for line in LOGO.lines().skip(1) {
        eprintln!("{}", line.cyan());
    }
    eprintln!();

    // Top border
    let top = format!("  ╔{}╗", "═".repeat(BOX_WIDTH));
    let bot = format!("  ╚{}╝", "═".repeat(BOX_WIDTH));

    eprintln!("{}", top.dimmed());
    for line in WELCOME_LINES {
        // Pad each line to BOX_WIDTH using char count (not byte count)
        let char_len = line.chars().count();
        let padding = if char_len < BOX_WIDTH {
            BOX_WIDTH - char_len
        } else {
            0
        };
        eprintln!(
            "  {}{}{}{}",
            "║".dimmed(),
            line,
            " ".repeat(padding),
            "║".dimmed()
        );
    }
    eprintln!("{}", bot.dimmed());
    eprintln!();
}
