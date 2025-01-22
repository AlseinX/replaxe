use std::{
    borrow::Cow,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use clap::Parser;
use encoding::EncoderTrap;

use crate::{input::input, r#match::Matches};

#[derive(Debug, Parser)]
pub struct Job {
    /// Files to search and replace in. Supports glob patterns.
    files: Vec<PathBuf>,

    /// (Would be prompted if not specified) Pattern to search for, simply use `*` as a wildcard to match any string that could be referred from the replace pattern. Wildcards would be matched as short as possible.
    #[clap(short, long)]
    pattern: Option<String>,

    /// (Would be prompted if not specified) Replace pattern, use `*` to refer to the nth captured group in the pattern.
    #[clap(short, long)]
    replace: Option<String>,

    /// (Would be prompted if not specified) Reorder the captured groups in the replace pattern, use `,` or `;` or ` ` or `\t` to separate the indexes. Indices that are not specified will be left in their original order.
    #[clap(short = 'o', long)]
    reorder: Option<String>,

    /// Skip the confirmation prompt before actually overwriting the files.
    #[clap(short, long)]
    yes: bool,

    /// Override the wildcard character instead of `*`.
    #[clap(short, long, default_value = "*")]
    wildcard: String,

    /// Enable multiline input mode when prompting for patterns and replaces, type an empty line to submit the input.
    #[clap(short, long)]
    multiline: bool,
}

impl Job {
    pub fn execute(mut self) -> Result<()> {
        self.expand_globs();

        let pattern = if let Some(pattern) = self.pattern.as_deref() {
            Cow::Borrowed(pattern)
        } else {
            Cow::Owned(input("pattern", self.multiline)?)
        };

        let files = self
            .files
            .iter()
            .map(|f| {
                fs::read(f)
                    .map_err(Into::into)
                    .and_then(|file| Self::handle_file(f, file, &pattern, &self.wildcard))
                    .map(|(m, e)| (f.as_path(), m, e))
            })
            .collect::<Result<Vec<_>, _>>()?;

        if !files.iter().any(|(_, m, _)| !m.is_empty()) {
            return Err(anyhow!("no matches found"));
        }

        for (path, file, encoding) in &files {
            println!(
                "file: \"{}\", encoding: \"{}\"\n{file}",
                path.display(),
                encoding
            );
        }

        let replace = if let Some(replace) = self.replace.as_deref() {
            Cow::Borrowed(replace)
        } else {
            Cow::Owned(input("replace", self.multiline)?)
        };

        let reorder = if let Some(reorder) = self.reorder.as_deref() {
            Cow::Borrowed(reorder)
        } else {
            Cow::Owned(input("reorder", false)?)
        };

        let reorder = if reorder.is_empty() {
            Vec::new()
        } else {
            reorder
                .split([',', ' ', ';', '\t'])
                .map(|x| x.parse().map(|x: usize| x - 1))
                .collect::<Result<Vec<_>, _>>()?
        };

        let results = files
            .iter()
            .map(|(path, matches, encoding)| {
                matches
                    .replace(&replace, &self.wildcard, reorder.iter().copied())
                    .and_then(|m| {
                        charset_normalizer_rs::utils::encode(
                            m.text(),
                            encoding,
                            EncoderTrap::Strict,
                        )
                        .map_err(|_| {
                            anyhow!(
                                "\"{}\": replaced text is not valid in the original encoding \"{encoding}\"",
                                path.display()
                            )
                        })
                        .map(|d| (m, d))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;

        for ((path, _, encoding), (result, _)) in std::iter::zip(&files, &results) {
            println!(
                "file: \"{}\", encoding: \"{}\"\n{result}",
                path.display(),
                encoding
            );
        }

        if !self.yes {
            input("press enter to confirm", false)?;
        }

        for ((path, _, _), (_, data)) in std::iter::zip(files, results) {
            fs::write(path, data)?;
        }

        Ok(())
    }

    fn handle_file(
        path: &Path,
        file: Vec<u8>,
        pattern: &str,
        wildcard: &str,
    ) -> Result<(Matches, String)> {
        let file = charset_normalizer_rs::from_bytes(&file, None);
        let guess = file
            .get_best()
            .ok_or_else(|| anyhow!("\"{}\": unknown encoding", path.display()))?;
        let content = guess
            .decoded_payload()
            .ok_or_else(|| anyhow!("\"{}\": unknown encoding", path.display()))?;
        let encoding = guess.encoding().to_owned();
        let matches = Matches::new(content, pattern, wildcard)?;
        Ok((matches, encoding))
    }

    fn expand_globs(&mut self) {
        let mut expanded = Vec::with_capacity(self.files.len());
        for f in self.files.drain(..) {
            match glob::glob(&f.as_os_str().to_string_lossy()) {
                Ok(paths) => {
                    for path in paths.flatten() {
                        expanded.push(path);
                    }
                }
                Err(_) => expanded.push(f),
            }
        }
        self.files = expanded;
    }
}
