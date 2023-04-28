use anyhow::{anyhow, Error};
use std::io::BufRead;
use std::sync::mpsc;

// TODO implement placeholder

fn main() -> Result<(), Error> {
    let (base, prompt) = parse_args()?;
    let (tx, rx) = mpsc::channel::<Vec<String>>();

    let mut readline = rustyline::Editor::new()?;
    let printer = readline.create_external_printer()?;

    let mut runner = Runner { base, rx, printer };
    let interceptor = Interceptor { tx };

    std::thread::spawn(move || runner.run().unwrap());
    readline.set_helper(Some(interceptor));

    let mut latest = String::new();
    loop {
        latest = readline.readline_with_initial(&prompt, (&latest, ""))?;
    }
}

//// parse command and invariant args from ARGV

fn parse_args() -> Result<(CommandBase, String), Error> {
    let mut argv = std::env::args();
    let _ = argv.next().ok_or_else(|| anyhow!("missing argv[0]"))?;

    let program = argv.next().ok_or_else(|| anyhow!("missing argv[1]"))?;
    let args: Vec<String> = argv.collect();

    let prompt = format!("> {} {}", shell_words::quote(&program), shell_words::join(&args));

    Ok((CommandBase { program, args }, prompt))
}

struct CommandBase {
    program: String,
    args: Vec<String>,
}

//// background runner

pub struct Runner<P> {
    base: CommandBase,
    rx: mpsc::Receiver<Vec<String>>,
    printer: P,
}

// naturally prints on panic, then the main thread panics on next send
impl<P: rustyline::ExternalPrinter> Runner<P> {
    const RESET: &str = "\x1B[2J\x1B[1;1H";

    fn run(&mut self) -> Result<(), Error> {
        let mut args = Vec::new();

        while let Ok(next_args) = self.rx.recv() {
            if next_args != args {
                args = next_args;

                // TODO interleave stdout + stderr?
                let output = std::process::Command::new(&self.base.program)
                    .args(&self.base.args)
                    .args(&args)
                    .output()?;

                let mut screen_reset = true;
                self.print_out(&output.stderr, &mut screen_reset)?;
                self.print_out(&output.stdout, &mut screen_reset)?;
                if screen_reset {
                    self.printer.print(String::from(Self::RESET))?;
                }
            }
        }

        Ok(())
    }

    fn print_out(&mut self, out: &[u8], reset: &mut bool) -> Result<(), Error> {
        for res_line in std::io::Cursor::new(out).lines() {
            let reset_prefix = if *reset {
                *reset = false;
                Self::RESET
            } else {
                ""
            };

            match res_line {
                Ok(line) => {
                    self.printer.print(format!("{}{}", reset_prefix, line))?;
                }
                Err(err) => {
                    self.printer.print(format!("{}cursor.lines() error: {}", reset_prefix, err))?;
                }
            }
        }
        Ok(())
    }
}

//// readline interceptor

struct Interceptor {
    tx: mpsc::Sender<Vec<String>>,
}

impl rustyline::hint::Hinter for Interceptor {
    type Hint = String;

    fn hint(&self, line: &str, _pos: usize, _ctx: &rustyline::Context<'_>) -> Option<String> {
        match shell_words::split(line) {
            Ok(args) => {
                self.tx.send(args).unwrap();
                None
            }
            Err(parse_err) => Some(format!("  ({})", parse_err))
        }
    }
}

impl rustyline::completion::Completer for Interceptor {
    type Candidate = String;
}
impl rustyline::highlight::Highlighter for Interceptor {}
impl rustyline::validate::Validator for Interceptor {
// this doesn't work great, TODO investigate
//    fn validate(
//        &self,
//        _ctx: &mut rustyline::validate::ValidationContext<'_>,
//    ) -> rustyline::Result<rustyline::validate::ValidationResult> {
//        Ok(rustyline::validate::ValidationResult::Invalid(None))
//    }
}
impl rustyline::Helper for Interceptor {}
