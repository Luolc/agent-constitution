//! `statute constitution build`: builds or checks the constitution of the
//! checkout.

use std::path::{Path, PathBuf};

use super::{AGENT, BUILD_COMMAND, Errors, Sources, base, check, load, render};

#[derive(clap::Subcommand)]
pub enum Action {
    /// Check the constitution's sources and write its generated files: the
    /// full version and the version for agents in each language.
    Build(BuildArgs),
}

#[derive(clap::Args)]
pub struct BuildArgs {
    /// Write nothing; also fail when the committed generated files differ
    /// from a fresh build.
    #[arg(long)]
    pub check: bool,
    /// With --check: also compare the numbering and the revision with this
    /// commit.
    #[arg(long, value_name = "GIT_REF", requires = "check")]
    pub base: Option<String>,
    /// The repository's root.
    #[arg(long, default_value = ".")]
    pub root: PathBuf,
}

/// Runs the action and prints its errors, one per line, to standard
/// error.
pub fn run(action: Action) -> Result<(), Errors> {
    let Action::Build(args) = action;
    let result = build_or_check(&args.root, args.check, args.base.as_deref());
    if let Err(errors) = &result {
        for e in &errors.0 {
            eprintln!("{e}");
        }
    }
    result
}

/// The generated files, (name, text), the name the language with
/// [`AGENT`] after it for the version for agents: the full versions in
/// the order of `index.toml`, then the versions for agents.
pub type Outputs = Vec<(String, String)>;

/// Checks 1 to 5, stopping after the first that fails, then renders.
pub fn build(root: &Path) -> Result<(Sources, Outputs), Errors> {
    let src = load(root)?;
    let errors = check(&src);
    if !errors.is_empty() {
        return Err(errors);
    }
    let mut outputs: Outputs = src
        .languages
        .iter()
        .map(|lang| (lang.clone(), render(lang, &src, false)))
        .collect();
    outputs.extend(
        src.languages
            .iter()
            .map(|lang| (format!("{lang}{AGENT}"), render(lang, &src, true))),
    );
    Ok((src, outputs))
}

/// Check 6: the committed outputs equal a fresh build byte for byte.
pub fn check_outputs(root: &Path, src: &Sources, outputs: &Outputs, errors: &mut Errors) {
    for (name, text) in outputs {
        let path = src.layout.output(name);
        let Ok(committed) = std::fs::read(root.join(&path)) else {
            errors.add(&path, 1, 6, format!("missing; run `{BUILD_COMMAND}`"));
            continue;
        };
        if committed == text.as_bytes() {
            continue;
        }
        let old: Vec<&[u8]> = committed.split(|&b| b == b'\n').collect();
        let new: Vec<&[u8]> = text.as_bytes().split(|&b| b == b'\n').collect();
        // When the shorter is a prefix of the longer, the line reported is
        // the shorter one's last.
        let line = old
            .iter()
            .zip(&new)
            .position(|(a, b)| a != b)
            .map_or(old.len().min(new.len()), |i| i + 1);
        errors.add(
            &path,
            line,
            6,
            format!("differs from a fresh build; run `{BUILD_COMMAND}`"),
        );
    }
}

/// Builds the constitution at `root` and writes the outputs, or with
/// `check` checks that the committed ones are fresh, and with `base`
/// compares with that commit. A note of a skipped comparison goes to
/// standard output.
pub fn build_or_check(root: &Path, check: bool, base_ref: Option<&str>) -> Result<(), Errors> {
    let (src, outputs) = build(root)?;
    let mut errors = Errors::default();
    if check {
        check_outputs(root, &src, &outputs, &mut errors);
    }
    if errors.is_empty()
        && let Some(reference) = base_ref
    {
        match base::check_base(root, reference, &src, &mut errors) {
            Ok(Some(note)) => println!("{note}"),
            Ok(None) => {}
            Err(base::Unreadable(e)) => errors.add(src.layout.index(), 1, 7, e),
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    if !check {
        for (name, text) in &outputs {
            let path = root.join(src.layout.output(name));
            let written = path
                .parent()
                .map_or(Ok(()), std::fs::create_dir_all)
                .and_then(|()| std::fs::write(&path, text));
            if let Err(e) = written {
                errors.add(src.layout.output(name), 1, 6, format!("cannot write: {e}"));
                return Err(errors);
            }
        }
    }
    Ok(())
}
