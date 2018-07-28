use std::fmt::Display;
use std::fmt::Write as FmtWrite;
use std::io::{self, BufRead, Stdin, Write};

use yansi::Paint;

macro_rules! print_flush {
    ($($arg:tt)*) => {
        let out = io::stdout();
        let mut lock = out.lock();
        let _ = lock.write_fmt(format_args!($($arg)*));
        let _ = lock.flush();
    };
}

fn join_displays<I, D>(glue: &str, pieces: I) -> String
where
    I: IntoIterator<Item = D>,
    D: Display,
{
    let mut buf = String::new();
    let mut iter = pieces.into_iter().peekable();
    loop {
        match iter.next() {
            Some(piece) => {
                let _ = write!(buf, "{}", piece);
                if iter.peek().is_some() {
                    write!(buf, "{}", glue);
                }
            }
            None => break,
        }
    }
    buf
}

fn style_default<D>(disp: D) -> Paint<D>
where
    D: Display,
{
    Paint::new(disp).underline().bold()
}

pub struct Input {
    inner: Stdin,
}

impl Input {
    pub fn new() -> Input {
        Input { inner: io::stdin() }
    }

    pub fn read_line(&self) -> String {
        let mut lock = self.inner.lock();
        let mut buf = String::new();
        let _ = lock.read_line(&mut buf);
        buf.truncate(buf.trim().len());
        buf
    }

    pub fn ask_line(&self, question: &str) -> String {
        print_flush!("{} ", question);
        self.read_line()
    }

    pub fn confirm(&self, question: &str, default: Option<bool>) -> bool {
        let (yes, no) = match default {
            Some(true) => (style_default("y"), Paint::new("n")),
            Some(false) => (Paint::new("y"), style_default("n")),
            None => (Paint::new("y"), Paint::new("n")),
        };

        loop {
            print_flush!("{} [{}/{}]: ", question, yes, no);
            let line = self.read_line();
            match line.as_str() {
                "" if default.is_some() => return default.unwrap(),
                "y" | "Y" => return true,
                "n" | "N" => return false,
                _ => {}
            }
        }
    }

    pub fn select<'c, 'n>(
        &self,
        question: &str,
        choices: impl AsRef<[(&'c str, &'n str)]>,
        default: Option<&'c str>,
    ) -> &'c str {
        let choices = choices.as_ref();
        let codes: Vec<&str> = choices.iter().map(|&(code, _)| code).collect();

        let codes_str = join_displays(
            "/",
            choices.iter().map(|&(code, _)| {
                if Some(code) == default {
                    style_default(code)
                } else {
                    Paint::new(code)
                }.to_string()
            }),
        );
        let names_str = join_displays(
            ", ",
            choices.iter().map(|&(code, name)| {
                if Some(code) == default {
                    style_default(name)
                } else {
                    Paint::new(name)
                }.to_string()
            }),
        );

        loop {
            print_flush!("{} ({}) [{}]: ", question, names_str, codes_str);
            let line = self.read_line();
            if let (true, Some(default)) = (line.is_empty(), default) {
                return default;
            }
            for &code in codes.iter() {
                if line.as_str() == code {
                    return code;
                }
            }
        }
    }
}
