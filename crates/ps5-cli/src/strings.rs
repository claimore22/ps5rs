use std::path::PathBuf;

use crate::util::{load_file, write_to_output_or_stdout};

pub(crate) fn cmd_strings(path: &PathBuf, min_length: u8, offsets: bool, output: &Option<PathBuf>) {
    let data = load_file(path);
    let min = min_length as usize;

    write_to_output_or_stdout(output, &|w| {
        let mut start: Option<usize> = None;

        for (i, &byte) in data.iter().enumerate() {
            let printable = byte.is_ascii_graphic() || byte == b' ';
            if printable {
                if start.is_none() {
                    start = Some(i);
                }
            } else if let Some(s) = start {
                if i - s >= min
                    && let Ok(string) = std::str::from_utf8(&data[s..i])
                {
                    if offsets {
                        writeln!(w, "{:08x} {string}", s)?;
                    } else {
                        writeln!(w, "{string}")?;
                    }
                }
                start = None;
            }
        }

        if let Some(s) = start
            && data.len() - s >= min
            && let Ok(string) = std::str::from_utf8(&data[s..])
        {
            if offsets {
                writeln!(w, "{:08x} {string}", s)?;
            } else {
                writeln!(w, "{string}")?;
            }
        }

        Ok(())
    });
}
