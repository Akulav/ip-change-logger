//! Console build of IPTrail for Windows terminals.
//!
//! `IPTrail.exe` uses the GUI subsystem, so shells don't wait for it and its
//! output can land after the prompt. This binary is identical but always
//! console-attached, which suits `--once`, `--headless` and scripting.

fn main() -> std::process::ExitCode {
    iptrail::run()
}
