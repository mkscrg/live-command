use anyhow::{anyhow, Error};
use std::sync::mpsc;
use std::thread;

fn main() -> Result<(), Error> {
    let (base, prompt) = parse_args()?;
    let (tx, rx) = mpsc::channel::<Vec<String>>();

    let mut readline = rustyline::Editor::new()?;
    let printer = readline.create_external_printer()?;

    let mut runner = Runner { base, rx, printer };
    let interceptor = Interceptor { tx };

    thread::spawn(move || runner.run());
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

// TODO what happens when this crashes
impl<P: rustyline::ExternalPrinter> Runner<P> {
    fn run(&mut self) {
        let mut last_args = Vec::new();

        while let Ok(next_args) = self.rx.recv() {
            if next_args != last_args {
                self.printer
                    .print(format!(
                        "\x1B[2J\x1B[1;1Hwould run: {} {} {}",
                        shell_words::quote(&self.base.program),
                        shell_words::join(&self.base.args),
                        shell_words::join(&next_args),
                    ))
                    .unwrap();
                // TODO actually run commands
            }
            last_args = next_args;
        }
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
