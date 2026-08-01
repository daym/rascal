use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

const DEFAULT_SOURCE: &str = "tests";
const DEFAULT_OUTPUT: &str = "tests/pascal/generated";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PascalKind {
    Unit,
    Program,
    Library,
    Package,
    BareProgram,
}

impl PascalKind {
    const fn name(self) -> &'static str {
        match self {
            Self::Unit => "unit",
            Self::Program => "program",
            Self::Library => "library",
            Self::Package => "package",
            Self::BareProgram => "body",
        }
    }
}

#[derive(Debug)]
struct StringGroup {
    text: String,
    start: usize,
}

#[derive(Debug)]
struct Candidate {
    source_path: PathBuf,
    source_line: usize,
    test_name: String,
    kind: PascalKind,
    declared_name: Option<String>,
    text: String,
}

#[derive(Debug)]
struct Fixture {
    relative_path: PathBuf,
    source_path: PathBuf,
    source_line: usize,
    test_name: String,
    kind: PascalKind,
    text: String,
}

#[derive(Clone, Copy, Debug)]
struct LiteralStart {
    quote: usize,
    raw: bool,
}

fn is_ident_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

fn literal_start_at(source: &[u8], index: usize) -> Option<LiteralStart> {
    if index >= source.len() {
        return None;
    }

    let candidates: &[(&[u8], bool, usize)] = &[
        (b"u8R\"", true, 3),
        (b"uR\"", true, 2),
        (b"UR\"", true, 2),
        (b"LR\"", true, 2),
        (b"R\"", true, 1),
        (b"u8\"", false, 2),
        (b"u\"", false, 1),
        (b"U\"", false, 1),
        (b"L\"", false, 1),
        (b"\"", false, 0),
    ];

    for &(prefix, raw, quote_offset) in candidates {
        if source[index..].starts_with(prefix)
            && (quote_offset == 0 || index == 0 || !is_ident_byte(source[index.saturating_sub(1)]))
        {
            return Some(LiteralStart {
                quote: index + quote_offset,
                raw,
            });
        }
    }
    None
}

fn hex_value(byte: u8) -> Option<u32> {
    match byte {
        b'0'..=b'9' => Some(u32::from(byte - b'0')),
        b'a'..=b'f' => Some(u32::from(byte - b'a') + 10),
        b'A'..=b'F' => Some(u32::from(byte - b'A') + 10),
        _ => None,
    }
}

fn push_codepoint(output: &mut String, value: u32) -> Result<(), String> {
    let character =
        char::from_u32(value).ok_or_else(|| format!("invalid Unicode escape U+{value:04X}"))?;
    output.push(character);
    Ok(())
}

fn parse_ordinary_literal(source: &[u8], quote: usize) -> Result<(String, usize), String> {
    let mut output = String::new();
    let mut index = quote + 1;

    while index < source.len() {
        match source[index] {
            b'"' => return Ok((output, index + 1)),
            b'\\' => {
                index += 1;
                let Some(&escape) = source.get(index) else {
                    return Err("unterminated C++ escape".to_owned());
                };
                match escape {
                    b'\n' => {
                        index += 1;
                    }
                    b'\r' => {
                        index += 1;
                        if source.get(index) == Some(&b'\n') {
                            index += 1;
                        }
                    }
                    b'a' => {
                        output.push('\u{7}');
                        index += 1;
                    }
                    b'b' => {
                        output.push('\u{8}');
                        index += 1;
                    }
                    b'f' => {
                        output.push('\u{c}');
                        index += 1;
                    }
                    b'n' => {
                        output.push('\n');
                        index += 1;
                    }
                    b'r' => {
                        output.push('\r');
                        index += 1;
                    }
                    b't' => {
                        output.push('\t');
                        index += 1;
                    }
                    b'v' => {
                        output.push('\u{b}');
                        index += 1;
                    }
                    b'\\' => {
                        output.push('\\');
                        index += 1;
                    }
                    b'\'' => {
                        output.push('\'');
                        index += 1;
                    }
                    b'"' => {
                        output.push('"');
                        index += 1;
                    }
                    b'?' => {
                        output.push('?');
                        index += 1;
                    }
                    b'x' => {
                        index += 1;
                        let start = index;
                        let mut value = 0u32;
                        while let Some(digit) = source.get(index).and_then(|byte| hex_value(*byte))
                        {
                            value = value
                                .checked_mul(16)
                                .and_then(|value| value.checked_add(digit))
                                .ok_or_else(|| "C++ hexadecimal escape overflow".to_owned())?;
                            index += 1;
                        }
                        if index == start {
                            return Err("empty C++ hexadecimal escape".to_owned());
                        }
                        push_codepoint(&mut output, value)?;
                    }
                    b'u' | b'U' => {
                        let digits = if escape == b'u' { 4 } else { 8 };
                        index += 1;
                        let mut value = 0u32;
                        for _ in 0..digits {
                            let digit = source
                                .get(index)
                                .and_then(|byte| hex_value(*byte))
                                .ok_or_else(|| "short C++ Unicode escape".to_owned())?;
                            value = value * 16 + digit;
                            index += 1;
                        }
                        push_codepoint(&mut output, value)?;
                    }
                    b'0'..=b'7' => {
                        let mut value = 0u32;
                        let mut digits = 0;
                        while digits < 3 {
                            let Some(&byte @ b'0'..=b'7') = source.get(index) else {
                                break;
                            };
                            value = value * 8 + u32::from(byte - b'0');
                            digits += 1;
                            index += 1;
                        }
                        push_codepoint(&mut output, value)?;
                    }
                    other => {
                        output.push(char::from(other));
                        index += 1;
                    }
                }
            }
            byte if byte.is_ascii() => {
                output.push(char::from(byte));
                index += 1;
            }
            _ => {
                let tail = std::str::from_utf8(&source[index..])
                    .map_err(|error| format!("invalid UTF-8 in C++ literal: {error}"))?;
                let character = tail
                    .chars()
                    .next()
                    .ok_or_else(|| "unterminated UTF-8 sequence".to_owned())?;
                output.push(character);
                index += character.len_utf8();
            }
        }
    }

    Err("unterminated C++ string literal".to_owned())
}

fn parse_raw_literal(source: &[u8], quote: usize) -> Result<(String, usize), String> {
    let delimiter_start = quote + 1;
    let open = source[delimiter_start..]
        .iter()
        .position(|byte| *byte == b'(')
        .map(|offset| delimiter_start + offset)
        .ok_or_else(|| "unterminated C++ raw-string delimiter".to_owned())?;
    let delimiter = &source[delimiter_start..open];
    if delimiter.len() > 16 {
        return Err("C++ raw-string delimiter is longer than 16 bytes".to_owned());
    }

    let content_start = open + 1;
    let mut index = content_start;
    while index < source.len() {
        if source[index] == b')' {
            let delimiter_end = index + 1 + delimiter.len();
            if source.get(index + 1..delimiter_end) == Some(delimiter)
                && source.get(delimiter_end) == Some(&b'"')
            {
                let text = std::str::from_utf8(&source[content_start..index])
                    .map_err(|error| format!("invalid UTF-8 in C++ raw string: {error}"))?
                    .to_owned();
                return Ok((text, delimiter_end + 1));
            }
        }
        index += 1;
    }
    Err("unterminated C++ raw string".to_owned())
}

fn parse_literal(source: &[u8], index: usize) -> Result<Option<(String, usize)>, String> {
    let Some(start) = literal_start_at(source, index) else {
        return Ok(None);
    };
    let parsed = if start.raw {
        parse_raw_literal(source, start.quote)
    } else {
        parse_ordinary_literal(source, start.quote)
    }?;
    Ok(Some(parsed))
}

fn skip_cpp_trivia(source: &[u8], mut index: usize) -> usize {
    loop {
        while source.get(index).is_some_and(u8::is_ascii_whitespace) {
            index += 1;
        }
        if source.get(index..index + 2) == Some(b"//") {
            index += 2;
            while source.get(index).is_some_and(|byte| *byte != b'\n') {
                index += 1;
            }
            continue;
        }
        if source.get(index..index + 2) == Some(b"/*") {
            index += 2;
            while index + 1 < source.len() && &source[index..index + 2] != b"*/" {
                index += 1;
            }
            index = (index + 2).min(source.len());
            continue;
        }
        return index;
    }
}

fn skip_cpp_character(source: &[u8], mut index: usize) -> usize {
    index += 1;
    while index < source.len() {
        match source[index] {
            b'\\' => index = (index + 2).min(source.len()),
            b'\'' => return index + 1,
            _ => index += 1,
        }
    }
    index
}

fn extract_string_groups(source: &str) -> Result<Vec<StringGroup>, String> {
    let bytes = source.as_bytes();
    let mut groups = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        if bytes.get(index..index + 2) == Some(b"//") {
            index = skip_cpp_trivia(bytes, index);
            continue;
        }
        if bytes.get(index..index + 2) == Some(b"/*") {
            index = skip_cpp_trivia(bytes, index);
            continue;
        }
        if bytes[index] == b'\'' {
            index = skip_cpp_character(bytes, index);
            continue;
        }

        let Some((mut text, mut end)) =
            parse_literal(bytes, index).map_err(|error| format!("byte {index}: {error}"))?
        else {
            index += 1;
            continue;
        };
        let start = index;

        loop {
            let next = skip_cpp_trivia(bytes, end);
            let Some((suffix, suffix_end)) =
                parse_literal(bytes, next).map_err(|error| format!("byte {next}: {error}"))?
            else {
                break;
            };
            text.push_str(&suffix);
            end = suffix_end;
        }

        groups.push(StringGroup { text, start });
        index = end;
    }

    Ok(groups)
}

fn strip_pascal_prefix(mut source: &str) -> &str {
    loop {
        source = source.trim_start_matches(|character: char| character.is_whitespace());
        if let Some(rest) = source.strip_prefix('\u{feff}') {
            source = rest;
            continue;
        }
        if source.starts_with("{$")
            && let Some(end) = source.find('}')
        {
            source = &source[end + 1..];
            continue;
        }
        if source.starts_with("(*$")
            && let Some(end) = source.find("*)")
        {
            source = &source[end + 2..];
            continue;
        }
        if source.starts_with('{')
            && let Some(end) = source.find('}')
        {
            source = &source[end + 1..];
            continue;
        }
        if source.starts_with("(*")
            && let Some(end) = source.find("*)")
        {
            source = &source[end + 2..];
            continue;
        }
        return source;
    }
}

fn word_after(source: &str, keyword: &str) -> Option<String> {
    let source = source.trim_start();
    let tail = source.get(keyword.len()..)?.trim_start();
    let name: String = tail
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect();
    (!name.is_empty()).then(|| name.to_ascii_lowercase())
}

fn classify_pascal(source: &str) -> Option<(PascalKind, Option<String>)> {
    let significant = strip_pascal_prefix(source);
    let lower = significant.to_ascii_lowercase();
    let first_word: String = lower
        .chars()
        .take_while(|character| character.is_ascii_alphabetic())
        .collect();

    match first_word.as_str() {
        "unit"
            if (lower.contains("interface") || lower.contains("implementation"))
                && lower.contains("end.") =>
        {
            Some((PascalKind::Unit, word_after(significant, "unit")))
        }
        "program" if lower.contains("begin") && lower.contains("end.") => {
            Some((PascalKind::Program, word_after(significant, "program")))
        }
        "library" if lower.contains("begin") && lower.contains("end.") => {
            Some((PascalKind::Library, word_after(significant, "library")))
        }
        "package" if lower.contains("end.") => {
            Some((PascalKind::Package, word_after(significant, "package")))
        }
        "begin" if lower.contains("end.") => Some((PascalKind::BareProgram, None)),
        _ => None,
    }
}

fn source_line(source: &str, byte: usize) -> usize {
    source.as_bytes()[..byte]
        .iter()
        .filter(|byte| **byte == b'\n')
        .count()
        + 1
}

fn test_name_at(source: &str, byte: usize) -> String {
    let prefix = &source[..byte];
    let Some(start) = prefix.rfind("void test_") else {
        return "file_scope".to_owned();
    };
    let name_start = start + "void ".len();
    prefix[name_start..]
        .chars()
        .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
        .collect()
}

fn sanitize_name(name: &str) -> String {
    let mut output = String::new();
    let mut previous_separator = false;
    for character in name.chars() {
        let lowered = character.to_ascii_lowercase();
        if lowered.is_ascii_alphanumeric() {
            output.push(lowered);
            previous_separator = false;
        } else if !previous_separator {
            output.push('_');
            previous_separator = true;
        }
    }
    output.trim_matches('_').to_owned()
}

fn collect_candidates(source_root: &Path) -> Result<Vec<Candidate>, String> {
    let mut inputs = fs::read_dir(source_root)
        .map_err(|error| format!("cannot read {}: {error}", source_root.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "cc"))
        .collect::<Vec<_>>();
    inputs.sort();

    let mut candidates = Vec::new();
    for path in inputs {
        let source = fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        for group in extract_string_groups(&source)
            .map_err(|error| format!("cannot scan {}: {error}", path.display()))?
        {
            let Some((kind, declared_name)) = classify_pascal(&group.text) else {
                continue;
            };
            let test_name = test_name_at(&source, group.start);
            if test_name == "file_scope" {
                continue;
            }
            candidates.push(Candidate {
                source_path: path.clone(),
                source_line: source_line(&source, group.start),
                test_name,
                kind,
                declared_name,
                text: group.text,
            });
        }
    }
    Ok(candidates)
}

fn assign_fixture_paths(
    candidates: Vec<Candidate>,
    source_root: &Path,
) -> Result<Vec<Fixture>, String> {
    let mut counts = BTreeMap::<(PathBuf, String), usize>::new();
    for candidate in &candidates {
        *counts
            .entry((candidate.source_path.clone(), candidate.test_name.clone()))
            .or_default() += 1;
    }

    let mut positions = BTreeMap::<(PathBuf, String), usize>::new();
    let mut fixtures = Vec::new();
    let mut paths = BTreeSet::new();
    for candidate in candidates {
        let key = (candidate.source_path.clone(), candidate.test_name.clone());
        let position = positions.entry(key.clone()).or_default();
        *position += 1;
        let total = counts[&key];
        let test = sanitize_name(
            candidate
                .test_name
                .strip_prefix("test_")
                .unwrap_or(&candidate.test_name),
        );
        let suffix = if total == 1 {
            String::new()
        } else {
            format!(
                "__{:02}_{}{}",
                *position,
                candidate.kind.name(),
                candidate
                    .declared_name
                    .as_deref()
                    .map(|name| format!("_{}", sanitize_name(name)))
                    .unwrap_or_default()
            )
        };
        let source_stem = candidate
            .source_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| format!("invalid source path {}", candidate.source_path.display()))?;
        let relative_path = PathBuf::from(source_stem).join(format!("{test}{suffix}.pp"));
        if !paths.insert(relative_path.clone()) {
            return Err(format!("two fixtures map to {}", relative_path.display()));
        }
        fixtures.push(Fixture {
            relative_path,
            source_path: candidate
                .source_path
                .strip_prefix(source_root.parent().unwrap_or(Path::new("")))
                .unwrap_or(&candidate.source_path)
                .to_owned(),
            source_line: candidate.source_line,
            test_name: candidate.test_name,
            kind: candidate.kind,
            text: candidate.text,
        });
    }
    fixtures.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    Ok(fixtures)
}

fn manifest(fixtures: &[Fixture]) -> String {
    let mut output = "# generated path\tsource location\ttest function\tkind\tbytes\n".to_owned();
    for fixture in fixtures {
        output.push_str(&format!(
            "{}\t{}:{}\t{}\t{}\t{}\n",
            fixture.relative_path.display(),
            fixture.source_path.display(),
            fixture.source_line,
            fixture.test_name,
            fixture.kind.name(),
            fixture.text.len()
        ));
    }
    output
}

fn generated_files(root: &Path) -> Result<BTreeSet<PathBuf>, String> {
    fn visit(root: &Path, current: &Path, output: &mut BTreeSet<PathBuf>) -> Result<(), String> {
        for entry in fs::read_dir(current)
            .map_err(|error| format!("cannot read {}: {error}", current.display()))?
        {
            let entry = entry.map_err(|error| error.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                visit(root, &path, output)?;
            } else {
                output.insert(
                    path.strip_prefix(root)
                        .map_err(|error| error.to_string())?
                        .to_owned(),
                );
            }
        }
        Ok(())
    }

    let mut output = BTreeSet::new();
    if root.exists() {
        visit(root, root, &mut output)?;
    }
    Ok(output)
}

fn check_output(output_root: &Path, fixtures: &[Fixture]) -> Result<(), String> {
    let mut expected = fixtures
        .iter()
        .map(|fixture| fixture.relative_path.clone())
        .collect::<BTreeSet<_>>();
    expected.insert(PathBuf::from("manifest.tsv"));
    let actual = generated_files(output_root)?;
    if actual != expected {
        return Err(format!(
            "generated file set is stale\nexpected: {expected:#?}\nactual: {actual:#?}"
        ));
    }

    for fixture in fixtures {
        let path = output_root.join(&fixture.relative_path);
        let actual = fs::read_to_string(&path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        if actual != fixture.text {
            return Err(format!("{} is stale", path.display()));
        }
    }
    let manifest_path = output_root.join("manifest.tsv");
    let actual_manifest = fs::read_to_string(&manifest_path)
        .map_err(|error| format!("cannot read {}: {error}", manifest_path.display()))?;
    if actual_manifest != manifest(fixtures) {
        return Err(format!("{} is stale", manifest_path.display()));
    }
    Ok(())
}

fn write_output(output_root: &Path, fixtures: &[Fixture]) -> Result<(), String> {
    if output_root.exists() {
        fs::remove_dir_all(output_root)
            .map_err(|error| format!("cannot replace {}: {error}", output_root.display()))?;
    }
    fs::create_dir_all(output_root)
        .map_err(|error| format!("cannot create {}: {error}", output_root.display()))?;
    for fixture in fixtures {
        let path = output_root.join(&fixture.relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("cannot create {}: {error}", parent.display()))?;
        }
        fs::write(&path, &fixture.text)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
    }
    fs::write(output_root.join("manifest.tsv"), manifest(fixtures))
        .map_err(|error| format!("cannot write manifest: {error}"))?;
    Ok(())
}

fn run() -> Result<(), String> {
    let mut check = false;
    let mut source_root = PathBuf::from(DEFAULT_SOURCE);
    let mut output_root = PathBuf::from(DEFAULT_OUTPUT);
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--check" => check = true,
            "--source" => {
                source_root = PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| "--source requires a path".to_owned())?,
                );
            }
            "--output" => {
                output_root = PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| "--output requires a path".to_owned())?,
                );
            }
            "-h" | "--help" => {
                println!("usage: extract_pascal_tests [--check] [--source DIR] [--output DIR]");
                return Ok(());
            }
            other => return Err(format!("unknown argument `{other}`")),
        }
    }

    let candidates = collect_candidates(&source_root)?;
    let fixtures = assign_fixture_paths(candidates, &source_root)?;
    if fixtures.is_empty() {
        return Err(format!(
            "no complete Pascal sources found below {}",
            source_root.display()
        ));
    }

    if check {
        check_output(&output_root, &fixtures)?;
        println!("{} extracted Pascal fixtures are current", fixtures.len());
    } else {
        write_output(&output_root, &fixtures)?;
        println!(
            "wrote {} Pascal fixtures to {}",
            fixtures.len(),
            output_root.display()
        );
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("extract_pascal_tests: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_adjacent_cpp_literals_and_decodes_escapes() {
        let source = r#"void test_one() {
            parse("program p;\n" /* join */ "begin\nend.\n");
        }"#;
        let groups = extract_string_groups(source).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].text, "program p;\nbegin\nend.\n");
    }

    #[test]
    fn decodes_cpp_raw_literals() {
        let source = r#"void test_one() {
            parse(R"pas(unit u;
interface
implementation
end.
)pas");
        }"#;
        let groups = extract_string_groups(source).unwrap();
        assert_eq!(groups[0].text, "unit u;\ninterface\nimplementation\nend.\n");
    }

    #[test]
    fn classifies_directive_prefixed_units() {
        assert_eq!(
            classify_pascal("{$mode objfpc}\nunit U;\ninterface\nimplementation\nend.\n"),
            Some((PascalKind::Unit, Some("u".to_owned())))
        );
    }

    #[test]
    fn rejects_expected_cpp_fragments() {
        assert_eq!(classify_pascal("namespace p_u {"), None);
        assert_eq!(classify_pascal("void p_run();"), None);
        assert_eq!(classify_pascal("unit_name"), None);
    }
}
