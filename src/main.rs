use anyhow::{anyhow, Error};
use std::sync::mpsc;
use std::thread;

fn main() -> Result<(), Error> {
    let (base, prompt) = parse_args()?;
    let (tx, rx) = mpsc::channel::<String>();

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

//// arg parsing

struct CommandBase {
    program: String,
    args: Vec<String>,
}

fn parse_args() -> Result<(CommandBase, String), Error> {
    let mut argv = std::env::args();
    let _ = argv.next().ok_or_else(|| anyhow!("missing argv[0]"))?;

    let program = argv.next().ok_or_else(|| anyhow!("missing argv[1]"))?;
    let args: Vec<String> = argv.collect();

    // TODO drops quotes from cli, not ideal
    let prompt = format!("> {} {} ", &program, &args.join(" "));

    Ok((CommandBase { program, args }, prompt))
}

//// background runner

pub struct Runner<P> {
    base: CommandBase,
    rx: mpsc::Receiver<String>,
    printer: P,
}

// TODO what happens when this crashes
impl<P: rustyline::ExternalPrinter> Runner<P> {
    fn run(&mut self) {
        let mut last_line = String::new();

        while let Ok(next_line) = self.rx.recv() {
            if next_line != last_line {
                self.printer
                    .print(format!(
                        "\x1B[2J\x1B[1;1Hwould run: {} {} {}",
                        &self.base.program,
                        &self.base.args.join(" "),
                        next_line
                    ))
                    .unwrap();
                // TODO actually run commands
            }
            last_line = next_line;
        }
    }
}

//// readline interceptor

struct Interceptor {
    tx: mpsc::Sender<String>,
}

impl rustyline::hint::Hinter for Interceptor {
    // TODO parse args here, hint on open quotes, send only well-formed
    type Hint = String;

    fn hint(&self, line: &str, _pos: usize, _ctx: &rustyline::Context<'_>) -> Option<String> {
        self.tx.send(String::from(line)).unwrap();
        None
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
