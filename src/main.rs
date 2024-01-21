use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Ord, PartialOrd, PartialEq, Eq)]
enum Mode {
    Lines,
    Words,
    Chars,
    Bytes,
}

impl Mode {
    fn from_str(mode_str: &str) -> Option<Self> {
        match mode_str {
            "-l" => Some(Mode::Lines),
            "-w" => Some(Mode::Words),
            "-m" => Some(Mode::Chars),
            "-c" => Some(Mode::Bytes),
            _ => None,
        }
    }
}

struct BufferDetails {
    filename: Option<String>,
    buffer: Box<dyn BufRead>,
}

#[derive(Default)]
struct CountResult {
    summary: Option<String>,
    lines: u64,
    words: u64,
    chars: u64,
    bytes: u64,
}

impl CountResult {
    fn result_from_mode(&self, mode: &Mode) -> u64 {
        match mode {
            Mode::Lines => self.lines,
            Mode::Words => self.words,
            Mode::Chars => self.chars,
            Mode::Bytes => self.bytes,
        }
    }
}

fn count_buf(buffer_details: BufferDetails) -> CountResult {
    let mut buffer = buffer_details.buffer;
    let mut line_buf = Vec::<u8>::new();
    let mut count = CountResult {
        summary: buffer_details.filename,
        ..Default::default()
    };
    while buffer
        .read_until(b'\n', &mut line_buf)
        .expect("read_until failed")
        != 0
    {
        count.lines += 1;
        count.bytes += line_buf.len() as u64;

        // this moves the ownership of the read data to s
        // there is no allocation
        let s = String::from_utf8(line_buf).expect("from_utf8 failed");

        count.words += s.split_whitespace().count() as u64;
        count.chars += s.chars().count() as u64;

        // this returns the ownership of the read data to buf
        // there is no allocation
        line_buf = s.into_bytes();
        line_buf.clear();
    }
    count
}

fn sum_results(results: &Vec<CountResult>) -> CountResult {
    let mut total = CountResult {
        summary: Some("Total".into()),
        ..Default::default()
    };
    for result in results {
        total.lines += result.lines;
        total.words += result.words;
        total.chars += result.chars;
        total.bytes += result.bytes;
    }
    total
}

fn format_summary(mut results: Vec<CountResult>, mut modes: Vec<Mode>) -> String {
    // prepare modes for summary formatting
    modes.sort();
    modes.dedup();

    if results.len() > 1 {
        let total = sum_results(&results);
        results.push(total);
    }

    let output_counts: Vec<Vec<u64>> = results
        .iter()
        .map(|result| {
            modes
                .iter()
                .map(|mode| result.result_from_mode(mode))
                .collect()
        })
        .collect();

    let max_size = output_counts
        .iter()
        .flatten()
        .map(|count| count.to_string().len())
        .max()
        .expect("Max value guaranteed as vec is always populated");

    let output_counts_fmtd = output_counts.iter().map(|counts| {
        counts
            .iter()
            .map(|count| format!("{:>fill$}", count, fill = max_size))
            .collect::<Vec<_>>()
    });

    let output_summary = output_counts_fmtd
        .zip(results)
        .map(|(counts, results)| {
            format!(
                "{} {}",
                counts.join(" "),
                if let Some(summary) = results.summary {
                    summary
                } else {
                    "".into()
                }
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    output_summary
}

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args();
    args.next(); // ditch first arg, which is always process

    let mut modes: Vec<Mode> = vec![];
    let mut filenames: Vec<String> = vec![]; // if no filename is found then read from stdin

    for arg in args {
        let mode = Mode::from_str(&arg);
        match mode {
            Some(mode) => modes.push(mode),
            None => filenames.push(arg),
        }
    }

    if modes.len() == 0 {
        // default modes if no flags presented
        modes = vec![Mode::Lines, Mode::Words, Mode::Bytes];
    }

    let mut buffers: Vec<BufferDetails> = vec![];

    if filenames.len() == 0 {
        buffers.push(BufferDetails {
            filename: None,
            buffer: Box::new(std::io::stdin().lock()),
        })
    } else {
        for filename in filenames {
            let file = File::open(&filename)?;
            buffers.push(BufferDetails {
                filename: Some(filename),
                buffer: Box::new(BufReader::new(file)),
            })
        }
    };

    let results: Vec<_> = buffers
        .into_iter()
        .map(|buffer| count_buf(buffer))
        .collect();

    let summary = format_summary(results, modes);

    println!("{}", summary);

    Ok(())
}
