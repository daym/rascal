use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use rascal::{
    ApplicationType, AssemblerMode, LanguageFeature, LanguageMode, PreprocessorOptions,
    SourceMapEntryKind, TokenKind, lex, pascal_parser, preprocess,
    semantic::{bind_sources, bind_sources_with_options},
};

fn identifiers(output: &rascal::LexOutput) -> Vec<&str> {
    output
        .tokens
        .iter()
        .filter_map(|token| match &token.kind {
            TokenKind::Identifier(identifier) => Some(identifier.as_str()),
            _ => None,
        })
        .collect()
}

fn temporary_directory(label: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "rascal-preprocessor-{label}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn conditionals_define_values_and_ifopt_filter_the_token_stream() {
    let source = "
        {$define VERSION := 30200}
        {$define FEATURE}
        {$if VERSION >= 30200 and defined(FEATURE)}
          selected
        {$elseif defined(OTHER)}
          wrong_elseif
        {$else}
          wrong_else
        {$endif}

        {$H+}
        {$ifopt H+} long_strings {$else} wrong_ifopt {$endif}

        {$ifdef OUTER_IS_OFF}
          {$if AN_UNDEFINED_NAME > 1} dead_expression {$endif}
          ! this invalid token is inactive
        {$endif}
    ";
    let output = lex(source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(identifiers(&output), ["selected", "long_strings"]);
    assert!(
        output
            .source_map
            .iter()
            .any(|entry| entry.kind == SourceMapEntryKind::Inactive),
        "inactive source remains represented in the source map"
    );
}

#[test]
fn directive_state_is_snapshotted_and_push_pop_restores_it() {
    let output = lex("{$R+}{$Q+} checked \
         {$push}{$R-} range_off {$pop} checked_again");
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(output.tokens.len(), 3);
    assert!(output.tokens[0].modes.range_checks);
    assert!(output.tokens[0].modes.overflow_checks);
    assert!(!output.tokens[1].modes.range_checks);
    assert!(output.tokens[1].modes.overflow_checks);
    assert!(output.tokens[2].modes.range_checks);
    assert_eq!(
        output.tokens[0].directive_state,
        output.tokens[2].directive_state
    );
    assert_ne!(
        output.tokens[0].directive_state,
        output.tokens[1].directive_state
    );
}

#[test]
fn includes_are_textual_stateful_and_retain_physical_source_origins() {
    let directory = temporary_directory("include");
    let root = directory.join("main.pp");
    let include = directory.join("bound.inc");
    let nested_include = directory.join("number.inc");
    fs::write(&include, "{$Q+}{$I number.inc}").unwrap();
    fs::write(&nested_include, "7").unwrap();
    let source = "
        program Included;
        type Limit = 0..{$I bound.inc};
        var Value: LongInt;
        begin
          Value := 1 + 2;
        end.
    ";
    fs::write(&root, source).unwrap();
    let root_name = root.to_string_lossy();
    let output = preprocess(&root_name, source, &PreprocessorOptions::default());
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(output.sources.len(), 3);
    assert_eq!(output.dependencies.len(), 2);
    assert!(
        output
            .tokens
            .windows(2)
            .all(|tokens| tokens[0].span.end <= tokens[1].span.start),
        "parser-facing spans must remain monotonic through nested includes"
    );
    let included_integer = output
        .tokens
        .iter()
        .find(|token| token.kind == TokenKind::Integer(7))
        .unwrap();
    assert_ne!(included_integer.origin.source.as_u32(), 0);
    assert_eq!(output.physical_text(&included_integer.origin), Some("7"));
    assert_eq!(
        output.sources[included_integer.origin.source.as_u32() as usize].name,
        fs::canonicalize(&nested_include).unwrap().to_string_lossy()
    );
    assert_eq!(
        output.sources[included_integer.origin.source.as_u32() as usize]
            .line_column(included_integer.origin.range.start),
        Some((1, 1))
    );
    let plus = output
        .tokens
        .iter()
        .find(|token| token.kind == TokenKind::Plus)
        .unwrap();
    assert!(
        plus.modes.overflow_checks,
        "an include mutates the including token stream's directive state"
    );

    let parsed = pascal_parser::parse_named(&root_name, source);
    assert!(parsed.file.is_some());
    assert!(parsed.diagnostics.is_empty(), "{:#?}", parsed.diagnostics);
    let compilation = bind_sources(&[(&root_name, source)]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn command_line_defines_are_isolated_inputs_to_semantic_compilation() {
    let source = "
        program DefinedBuild;
        {$ifdef TARGET}
          var Selected: LongInt;
        {$else}
          this branch is deliberately not Pascal
        {$endif}
        begin
          Selected := 1;
        end.
    ";
    let without = bind_sources(&[("defined.pp", source)]);
    assert!(!without.diagnostics.is_empty());

    let mut options = PreprocessorOptions::default();
    options.define("target", "");
    let with = bind_sources_with_options(&[("defined.pp", source)], &options);
    assert!(with.diagnostics.is_empty(), "{:#?}", with.diagnostics);
}

#[test]
fn both_directive_comment_forms_and_deterministic_date_expansion_work() {
    let output = lex("(*$define ENABLED*) \
         {$if defined(ENABLED)} chosen {$endif} \
         const_stamp = {$I %DATE%}; \
         literal = '{$ifdef NOT_A_DIRECTIVE}';");
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert!(
        identifiers(&output).contains(&"chosen"),
        "tokens: {:#?}\ndirectives: {:#?}",
        output.tokens,
        output.directives
    );
    assert!(
        output
            .tokens
            .iter()
            .any(|token| matches!(&token.kind, TokenKind::String(value) if value == "1970-01-01"))
    );
    assert_eq!(
        output
            .directives
            .iter()
            .filter(|directive| directive.name == "ifdef")
            .count(),
        0,
        "directive-looking text inside a string is a string"
    );
}

#[test]
fn structural_preprocessor_errors_are_diagnosed_without_panicking() {
    let directory = temporary_directory("cycle");
    let root = directory.join("main.pp");
    let include = directory.join("cycle.inc");
    let root_source = "{$I cycle.inc} root";
    fs::write(&root, root_source).unwrap();
    fs::write(&include, "{$I main.pp} include").unwrap();
    let output = preprocess(
        &root.to_string_lossy(),
        root_source,
        &PreprocessorOptions::default(),
    );
    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("include cycle")),
        "{:#?}",
        output.diagnostics
    );

    let output = lex("{$else}{$endif}{$push}{$R+}{$ifdef X}");
    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("$else without")),
        "{:#?}",
        output.diagnostics
    );
    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("no matching $endif")),
        "{:#?}",
        output.diagnostics
    );
    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("no matching $pop")),
        "{:#?}",
        output.diagnostics
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn unknown_directives_are_retained_as_events_but_do_not_reach_pascal_syntax() {
    let output = lex("{$vendor_specific frobnicate} value");
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert_eq!(identifiers(&output), ["value"]);
    assert_eq!(output.directives.len(), 1);
    assert!(!output.directives[0].recognized);
}

#[test]
fn source_macros_are_recursive_textual_sources_with_physical_provenance() {
    let source = "
        program MacroBuild;
        {$macro on}
        {$define TInt := LongInt}
        {$define One := 1}
        {$define Two := One + 1}
        var Value: TInt;
        begin
          Value := Two;
        end.
    ";
    let output = lex(source);
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);
    assert!(!identifiers(&output).contains(&"tint"));
    assert!(!identifiers(&output).contains(&"one"));
    assert!(!identifiers(&output).contains(&"two"));
    assert!(identifiers(&output).contains(&"longint"));
    assert_eq!(
        output
            .macro_expansions
            .iter()
            .map(|expansion| expansion.name.as_str())
            .collect::<Vec<_>>(),
        ["tint", "two", "one"]
    );
    for expansion in &output.macro_expansions {
        let source = output.source(expansion.expanded_source).unwrap();
        assert!(source.synthetic);
        assert_eq!(source.included_from.as_ref(), Some(&expansion.invocation));
    }
    let expanded_one = output
        .tokens
        .iter()
        .find(|token| token.kind == TokenKind::Integer(1))
        .unwrap();
    assert_eq!(output.physical_text(&expanded_one.origin), Some("1"));
    assert!(
        output
            .source_map
            .iter()
            .any(|entry| entry.kind == SourceMapEntryKind::MacroInvocation)
    );
    assert!(
        output
            .tokens
            .windows(2)
            .all(|tokens| tokens[0].span.end <= tokens[1].span.start),
        "macro expansion must preserve monotonic parser-facing spans"
    );

    let compilation = bind_sources(&[("macro.pp", source)]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
}

#[test]
fn recursive_source_macros_are_diagnosed_and_stop_expanding() {
    let output = lex("
        {$macro on}
        {$define A := B}
        {$define B := A}
        A
    ");
    assert!(
        output
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("recursive source macro")),
        "{:#?}",
        output.diagnostics
    );
    assert_eq!(
        output
            .macro_expansions
            .iter()
            .map(|expansion| expansion.name.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    assert_eq!(identifiers(&output), ["a"]);
}

#[test]
fn language_profiles_and_build_directives_are_typed_state() {
    let output = lex("
        {$mode tp}
        turbo
        {$modeswitch advancedrecords+}
        records_on
        {$modeswitch advancedrecords-}
        records_off
        {$mode delphi}
        {$modeswitch macros+}
        {$warnings off}
        {$warn 6018 on}
        {$asmmode intel}
        {$apptype gui}
        delphi
    ");
    assert!(output.diagnostics.is_empty(), "{:#?}", output.diagnostics);

    let state_for = |name: &str| {
        let token = output
            .tokens
            .iter()
            .find(|token| token.kind == TokenKind::Identifier(name.to_owned()))
            .unwrap();
        output.directive_state(token.directive_state).unwrap()
    };
    let turbo = state_for("turbo");
    assert_eq!(turbo.language_mode, LanguageMode::TurboPascal);
    assert!(!turbo.feature_enabled(LanguageFeature::Classes));
    assert!(!turbo.feature_enabled(LanguageFeature::AdvancedRecords));

    let records_on = state_for("records_on");
    assert!(records_on.feature_enabled(LanguageFeature::AdvancedRecords));
    let records_off = state_for("records_off");
    assert!(!records_off.feature_enabled(LanguageFeature::AdvancedRecords));

    let delphi = state_for("delphi");
    assert_eq!(delphi.language_mode, LanguageMode::Delphi);
    assert!(delphi.feature_enabled(LanguageFeature::Classes));
    assert!(delphi.feature_enabled(LanguageFeature::InlineVariables));
    assert!(delphi.feature_enabled(LanguageFeature::Macros));
    assert!(!delphi.warnings_enabled);
    assert!(delphi.warning_enabled("6018"));
    assert!(!delphi.warning_enabled("unconfigured_warning"));
    assert_eq!(delphi.assembler_mode, AssemblerMode::Intel);
    assert_eq!(delphi.application_type, ApplicationType::Gui);
}

#[test]
fn semantic_files_retain_the_final_build_directive_state() {
    let source = "
        program BuildState;
        {$mode delphi}
        {$warnings off}
        {$warn SYMBOL_DEPRECATED on}
        {$asmmode intel}
        {$apptype console}
        begin
        end.
    ";
    let compilation = bind_sources(&[("build_state.pp", source)]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
    let state = &compilation.files[0].final_directive_state;
    assert_eq!(state.language_mode, LanguageMode::Delphi);
    assert!(!state.warnings_enabled);
    assert!(state.warning_enabled("symbol_deprecated"));
    assert_eq!(state.assembler_mode, AssemblerMode::Intel);
    assert_eq!(state.application_type, ApplicationType::Console);
}
