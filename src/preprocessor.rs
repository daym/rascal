use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use crate::{
    Diagnostic, ModeSnapshot, SourceId, SourceSpan, Span,
    lexer::{
        DirectiveEvent, IncludeDependency, LexOutput, MacroExpansion, RawLexeme, RawToken,
        SourceInfo, SourceMapEntry, SourceMapEntryKind, Token, TokenKind, lower_raw, raw_lex,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DirectiveStateId(u32);

impl DirectiveStateId {
    fn from_index(index: usize) -> Self {
        Self(u32::try_from(index).expect("directive-state registry exceeded u32::MAX entries"))
    }

    pub const fn as_u32(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InterfaceModel {
    #[default]
    Com,
    Corba,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum LanguageMode {
    Iso,
    Fpc,
    #[default]
    ObjFpc,
    TurboPascal,
    Delphi,
    DelphiUnicode,
    MacPas,
    Custom(String),
}

impl LanguageMode {
    fn parse(spelling: &str) -> Self {
        match canonical_identifier(spelling).as_str() {
            "iso" => Self::Iso,
            "fpc" => Self::Fpc,
            "objfpc" => Self::ObjFpc,
            "tp" | "turbopascal" => Self::TurboPascal,
            "delphi" => Self::Delphi,
            "delphiunicode" => Self::DelphiUnicode,
            "macpas" => Self::MacPas,
            custom => Self::Custom(custom.to_owned()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LanguageFeature {
    Classes,
    Interfaces,
    Exceptions,
    ResultIdentifier,
    OperatorOverloading,
    Generics,
    AdvancedRecords,
    InlineVariables,
    Macros,
}

impl LanguageFeature {
    const fn mode_switch_name(self) -> &'static str {
        match self {
            Self::Classes => "classes",
            Self::Interfaces => "interfaces",
            Self::Exceptions => "exceptions",
            Self::ResultIdentifier => "result",
            Self::OperatorOverloading => "operatoroverloading",
            Self::Generics => "generics",
            Self::AdvancedRecords => "advancedrecords",
            Self::InlineVariables => "inlinevariables",
            Self::Macros => "macros",
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum AssemblerMode {
    #[default]
    Default,
    Intel,
    Att,
    Motorola,
    Custom(String),
}

impl AssemblerMode {
    fn parse(spelling: &str) -> Self {
        match canonical_identifier(spelling).as_str() {
            "" | "default" => Self::Default,
            "intel" => Self::Intel,
            "att" => Self::Att,
            "motorola" => Self::Motorola,
            custom => Self::Custom(custom.to_owned()),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ApplicationType {
    #[default]
    Unspecified,
    Console,
    Gui,
    Library,
    Custom(String),
}

impl ApplicationType {
    fn parse(spelling: &str) -> Self {
        match canonical_identifier(spelling).as_str() {
            "" | "default" => Self::Unspecified,
            "console" => Self::Console,
            "gui" => Self::Gui,
            "library" => Self::Library,
            custom => Self::Custom(custom.to_owned()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectiveState {
    pub modes: ModeSnapshot,
    pub long_strings: bool,
    pub typed_addresses: bool,
    pub assertions: bool,
    pub interface_model: InterfaceModel,
    pub language_mode: LanguageMode,
    pub mode_switches: BTreeMap<String, bool>,
    pub macros_enabled: bool,
    pub warnings_enabled: bool,
    pub warning_controls: BTreeMap<String, bool>,
    pub assembler_mode: AssemblerMode,
    pub application_type: ApplicationType,
    pub record_packing: Option<String>,
    pub enum_packing: Option<String>,
    pub set_packing: Option<String>,
    pub other_switches: BTreeMap<char, bool>,
}

impl Default for DirectiveState {
    fn default() -> Self {
        Self {
            modes: ModeSnapshot::default(),
            long_strings: false,
            typed_addresses: false,
            assertions: false,
            interface_model: InterfaceModel::Com,
            language_mode: LanguageMode::ObjFpc,
            mode_switches: BTreeMap::new(),
            macros_enabled: false,
            warnings_enabled: true,
            warning_controls: BTreeMap::new(),
            assembler_mode: AssemblerMode::Default,
            application_type: ApplicationType::Unspecified,
            record_packing: None,
            enum_packing: None,
            set_packing: None,
            other_switches: BTreeMap::new(),
        }
    }
}

impl DirectiveState {
    pub fn feature_enabled(&self, feature: LanguageFeature) -> bool {
        if let Some(enabled) = self.mode_switches.get(feature.mode_switch_name()) {
            return *enabled;
        }
        if feature == LanguageFeature::Macros {
            return self.macros_enabled;
        }
        match self.language_mode {
            LanguageMode::Iso | LanguageMode::TurboPascal | LanguageMode::MacPas => false,
            LanguageMode::Fpc => matches!(
                feature,
                LanguageFeature::Classes
                    | LanguageFeature::Interfaces
                    | LanguageFeature::Exceptions
                    | LanguageFeature::OperatorOverloading
            ),
            LanguageMode::ObjFpc => matches!(
                feature,
                LanguageFeature::Classes
                    | LanguageFeature::Interfaces
                    | LanguageFeature::Exceptions
                    | LanguageFeature::ResultIdentifier
                    | LanguageFeature::OperatorOverloading
                    | LanguageFeature::Generics
            ),
            LanguageMode::Delphi | LanguageMode::DelphiUnicode => {
                !matches!(feature, LanguageFeature::Macros)
            }
            LanguageMode::Custom(_) => false,
        }
    }

    pub fn warning_enabled(&self, warning: &str) -> bool {
        self.warning_controls
            .get(&canonical_identifier(warning))
            .copied()
            .unwrap_or(self.warnings_enabled)
    }

    pub fn switch_enabled(&self, letter: char) -> bool {
        match letter.to_ascii_lowercase() {
            'b' => self.modes.complete_boolean_eval,
            'h' => self.long_strings,
            'i' => self.modes.io_checks,
            'q' => self.modes.overflow_checks,
            'r' => self.modes.range_checks,
            't' => self.typed_addresses,
            'v' => self.modes.var_string_checks,
            letter => self.other_switches.get(&letter).copied().unwrap_or(false),
        }
    }

    fn set_switch(&mut self, letter: char, enabled: bool) {
        match letter.to_ascii_lowercase() {
            'b' => self.modes.complete_boolean_eval = enabled,
            'h' => self.long_strings = enabled,
            'i' => self.modes.io_checks = enabled,
            'q' => self.modes.overflow_checks = enabled,
            'r' => self.modes.range_checks = enabled,
            't' => self.typed_addresses = enabled,
            'v' => self.modes.var_string_checks = enabled,
            letter => {
                self.other_switches.insert(letter, enabled);
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct PreprocessorOptions {
    pub defines: BTreeMap<String, String>,
    pub include_paths: Vec<PathBuf>,
    pub max_include_depth: usize,
    pub max_macro_expansion_depth: usize,
    /// Text substituted by `{$I %DATE%}`. Keeping this injected makes builds
    /// reproducible; callers may deliberately provide another value.
    pub date_macro: String,
}

impl Default for PreprocessorOptions {
    fn default() -> Self {
        Self {
            defines: BTreeMap::new(),
            include_paths: Vec::new(),
            max_include_depth: 64,
            max_macro_expansion_depth: 64,
            date_macro: "1970-01-01".to_owned(),
        }
    }
}

impl PreprocessorOptions {
    pub fn define(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.defines
            .insert(canonical_identifier(&name.into()), value.into());
    }
}

#[derive(Clone, Debug)]
struct SourceBuffer {
    info: SourceInfo,
    raw: Vec<RawLexeme>,
    path: Option<PathBuf>,
}

#[derive(Clone, Debug)]
struct SourceFrame {
    source: SourceId,
    next: usize,
    physical_cursor: usize,
    active_path: Option<PathBuf>,
    active_macro: Option<String>,
}

#[derive(Clone, Debug)]
struct ConditionalFrame {
    outer_active: bool,
    branch_taken: bool,
    active: bool,
    saw_else: bool,
    origin: SourceSpan,
    logical_span: Span,
}

struct PreprocessorSession<'a> {
    options: &'a PreprocessorOptions,
    defines: BTreeMap<String, String>,
    state: DirectiveState,
    saved_states: Vec<(DirectiveState, SourceSpan, Span)>,
    conditionals: Vec<ConditionalFrame>,
    buffers: Vec<SourceBuffer>,
    sources_by_path: BTreeMap<PathBuf, SourceId>,
    active_paths: BTreeSet<PathBuf>,
    active_macros: BTreeSet<String>,
    frames: Vec<SourceFrame>,
    tokens: Vec<Token>,
    diagnostics: Vec<Diagnostic>,
    dependencies: Vec<IncludeDependency>,
    directives: Vec<DirectiveEvent>,
    macro_expansions: Vec<MacroExpansion>,
    source_map: Vec<SourceMapEntry>,
    directive_states: Vec<DirectiveState>,
    logical_cursor: usize,
}

impl<'a> PreprocessorSession<'a> {
    fn new(options: &'a PreprocessorOptions) -> Self {
        Self {
            options,
            defines: options
                .defines
                .iter()
                .map(|(name, value)| (canonical_identifier(name), value.clone()))
                .collect(),
            state: DirectiveState::default(),
            saved_states: Vec::new(),
            conditionals: Vec::new(),
            buffers: Vec::new(),
            sources_by_path: BTreeMap::new(),
            active_paths: BTreeSet::new(),
            active_macros: BTreeSet::new(),
            frames: Vec::new(),
            tokens: Vec::new(),
            diagnostics: Vec::new(),
            dependencies: Vec::new(),
            directives: Vec::new(),
            macro_expansions: Vec::new(),
            source_map: Vec::new(),
            directive_states: Vec::new(),
            logical_cursor: 0,
        }
    }

    fn add_source(
        &mut self,
        name: String,
        source: String,
        path: Option<PathBuf>,
        included_from: Option<SourceSpan>,
        synthetic: bool,
    ) -> SourceId {
        let id = SourceId::from_index(self.buffers.len());
        let byte_len = source.len();
        let raw = raw_lex(&source);
        let mut line_starts = vec![0];
        line_starts.extend(
            source
                .bytes()
                .enumerate()
                .filter_map(|(index, byte)| (byte == b'\n').then_some(index + 1)),
        );
        let info = SourceInfo {
            id,
            name,
            text: source,
            byte_len,
            line_starts,
            included_from,
            synthetic,
        };
        if let Some(path) = path.as_ref() {
            self.sources_by_path.insert(path.clone(), id);
        }
        self.buffers.push(SourceBuffer { info, raw, path });
        id
    }

    fn push_source(&mut self, source: SourceId) {
        self.push_source_with_macro(source, None);
    }

    fn push_source_with_macro(&mut self, source: SourceId, active_macro: Option<String>) {
        let active_path = self.buffers[source.as_u32() as usize].path.clone();
        if let Some(path) = active_path.as_ref() {
            self.active_paths.insert(path.clone());
        }
        if let Some(name) = active_macro.as_ref() {
            self.active_macros.insert(name.clone());
        }
        self.frames.push(SourceFrame {
            source,
            next: 0,
            physical_cursor: 0,
            active_path,
            active_macro,
        });
    }

    fn current_active(&self) -> bool {
        self.conditionals.last().is_none_or(|frame| frame.active)
    }

    fn source_name(&self, source: SourceId) -> &str {
        &self.buffers[source.as_u32() as usize].info.name
    }

    fn diagnostic(&mut self, logical_span: Span, origin: &SourceSpan, message: impl AsRef<str>) {
        self.diagnostics.push(Diagnostic::new(
            logical_span,
            format!(
                "{}:{}..{}: {}",
                self.source_name(origin.source),
                origin.range.start,
                origin.range.end,
                message.as_ref()
            ),
        ));
    }

    fn state_id(&mut self) -> DirectiveStateId {
        if let Some(index) = self
            .directive_states
            .iter()
            .position(|state| state == &self.state)
        {
            return DirectiveStateId::from_index(index);
        }
        let id = DirectiveStateId::from_index(self.directive_states.len());
        self.directive_states.push(self.state.clone());
        id
    }

    fn next_raw(&mut self) -> Option<(SourceId, RawLexeme, Span)> {
        loop {
            let frame = self.frames.last_mut()?;
            let buffer = &self.buffers[frame.source.as_u32() as usize];
            let Some(raw) = buffer.raw.get(frame.next).cloned() else {
                self.logical_cursor = self
                    .logical_cursor
                    .saturating_add(buffer.info.byte_len.saturating_sub(frame.physical_cursor));
                let frame = self.frames.pop().unwrap();
                if let Some(path) = frame.active_path {
                    self.active_paths.remove(&path);
                }
                if let Some(name) = frame.active_macro {
                    self.active_macros.remove(&name);
                }
                continue;
            };
            frame.next += 1;
            self.logical_cursor = self
                .logical_cursor
                .saturating_add(raw.span.start.saturating_sub(frame.physical_cursor));
            let logical_start = self.logical_cursor;
            self.logical_cursor = self.logical_cursor.saturating_add(raw.span.len());
            frame.physical_cursor = raw.span.end;
            return Some((frame.source, raw, logical_start..self.logical_cursor));
        }
    }

    fn run(mut self) -> LexOutput {
        while let Some((source, raw, logical_span)) = self.next_raw() {
            let origin = SourceSpan {
                source,
                range: raw.span.clone(),
            };
            match raw.token {
                Ok(RawToken::Directive(body)) => {
                    self.source_map.push(SourceMapEntry {
                        logical: logical_span.clone(),
                        physical: origin.clone(),
                        kind: SourceMapEntryKind::Directive,
                    });
                    self.handle_directive(&body, origin, logical_span);
                }
                Ok(RawToken::Identifier(name))
                    if self.current_active()
                        && self.state.feature_enabled(LanguageFeature::Macros)
                        && self.defines.contains_key(&name) =>
                {
                    if self.expand_macro(&name, &origin, &logical_span) {
                        self.source_map.push(SourceMapEntry {
                            logical: logical_span,
                            physical: origin,
                            kind: SourceMapEntryKind::MacroInvocation,
                        });
                    } else {
                        self.emit_raw_token(RawToken::Identifier(name), logical_span, origin);
                    }
                }
                Ok(raw) if self.current_active() => {
                    self.emit_raw_token(raw, logical_span, origin);
                }
                Ok(_) => {
                    self.source_map.push(SourceMapEntry {
                        logical: logical_span,
                        physical: origin,
                        kind: SourceMapEntryKind::Inactive,
                    });
                }
                Err(()) if self.current_active() => {
                    self.source_map.push(SourceMapEntry {
                        logical: logical_span.clone(),
                        physical: origin.clone(),
                        kind: SourceMapEntryKind::Invalid,
                    });
                    self.diagnostic(logical_span.clone(), &origin, "invalid source token");
                    let directive_state = self.state_id();
                    self.tokens.push(Token {
                        kind: TokenKind::Error,
                        span: logical_span,
                        origin,
                        modes: self.state.modes,
                        directive_state,
                    });
                }
                Err(()) => {
                    self.source_map.push(SourceMapEntry {
                        logical: logical_span,
                        physical: origin,
                        kind: SourceMapEntryKind::Inactive,
                    });
                }
            }
        }

        for frame in std::mem::take(&mut self.conditionals) {
            self.diagnostic(
                frame.logical_span,
                &frame.origin,
                "conditional directive has no matching $endif",
            );
        }
        for (_, origin, logical_span) in std::mem::take(&mut self.saved_states) {
            self.diagnostic(logical_span, &origin, "$push has no matching $pop");
        }
        let final_directive_state = self.state_id();
        LexOutput {
            tokens: self.tokens,
            diagnostics: self.diagnostics,
            sources: self.buffers.into_iter().map(|buffer| buffer.info).collect(),
            dependencies: self.dependencies,
            directives: self.directives,
            macro_expansions: self.macro_expansions,
            source_map: self.source_map,
            directive_states: self.directive_states,
            final_directive_state,
            logical_len: self.logical_cursor,
        }
    }

    fn emit_raw_token(&mut self, raw: RawToken, logical_span: Span, origin: SourceSpan) {
        if let Some(kind) = lower_raw(raw) {
            self.source_map.push(SourceMapEntry {
                logical: logical_span.clone(),
                physical: origin.clone(),
                kind: SourceMapEntryKind::Token,
            });
            let directive_state = self.state_id();
            self.tokens.push(Token {
                kind,
                span: logical_span,
                origin,
                modes: self.state.modes,
                directive_state,
            });
        }
    }

    fn handle_directive(&mut self, body: &str, origin: SourceSpan, logical_span: Span) {
        let (name, rest) = split_directive(body);
        let active_before = self.current_active();
        let recognized = self.apply_directive(&name, &rest, &origin, &logical_span);
        self.directives.push(DirectiveEvent {
            name,
            origin,
            active: active_before,
            recognized,
        });
    }

    fn apply_directive(
        &mut self,
        name: &str,
        rest: &str,
        origin: &SourceSpan,
        logical_span: &Span,
    ) -> bool {
        match name {
            "ifdef" | "ifndef" => {
                let outer_active = self.current_active();
                let mut condition = self
                    .defines
                    .contains_key(&canonical_identifier(rest.trim()));
                if name == "ifndef" {
                    condition = !condition;
                }
                let active = outer_active && condition;
                self.conditionals.push(ConditionalFrame {
                    outer_active,
                    branch_taken: active,
                    active,
                    saw_else: false,
                    origin: origin.clone(),
                    logical_span: logical_span.clone(),
                });
                return true;
            }
            "if" => {
                let outer_active = self.current_active();
                let condition =
                    outer_active && self.eval_condition(rest, origin, logical_span.clone());
                self.conditionals.push(ConditionalFrame {
                    outer_active,
                    branch_taken: condition,
                    active: condition,
                    saw_else: false,
                    origin: origin.clone(),
                    logical_span: logical_span.clone(),
                });
                return true;
            }
            "ifopt" => {
                let compact = compact_argument(rest);
                let condition = if compact.len() == 2 {
                    let mut chars = compact.chars();
                    let letter = chars.next().unwrap();
                    let requested = chars.next().unwrap();
                    if letter.is_ascii_alphabetic() && matches!(requested, '+' | '-') {
                        self.state.switch_enabled(letter) == (requested == '+')
                    } else {
                        self.diagnostic(
                            logical_span.clone(),
                            origin,
                            "$ifopt expects one option letter followed by + or -",
                        );
                        false
                    }
                } else {
                    self.diagnostic(
                        logical_span.clone(),
                        origin,
                        "$ifopt expects one option letter followed by + or -",
                    );
                    false
                };
                let outer_active = self.current_active();
                let active = outer_active && condition;
                self.conditionals.push(ConditionalFrame {
                    outer_active,
                    branch_taken: active,
                    active,
                    saw_else: false,
                    origin: origin.clone(),
                    logical_span: logical_span.clone(),
                });
                return true;
            }
            "elseif" => {
                let Some(index) = self.conditionals.len().checked_sub(1) else {
                    self.diagnostic(
                        logical_span.clone(),
                        origin,
                        "$elseif without matching conditional",
                    );
                    return true;
                };
                if self.conditionals[index].saw_else {
                    self.diagnostic(logical_span.clone(), origin, "$elseif after $else");
                    self.conditionals[index].active = false;
                    return true;
                }
                let should_evaluate =
                    self.conditionals[index].outer_active && !self.conditionals[index].branch_taken;
                let condition =
                    should_evaluate && self.eval_condition(rest, origin, logical_span.clone());
                let frame = &mut self.conditionals[index];
                frame.active = condition;
                frame.branch_taken |= condition;
                return true;
            }
            "else" => {
                let Some(index) = self.conditionals.len().checked_sub(1) else {
                    self.diagnostic(
                        logical_span.clone(),
                        origin,
                        "$else without matching conditional",
                    );
                    return true;
                };
                if self.conditionals[index].saw_else {
                    self.diagnostic(logical_span.clone(), origin, "duplicate $else");
                    self.conditionals[index].active = false;
                    return true;
                }
                let frame = &mut self.conditionals[index];
                frame.saw_else = true;
                frame.active = frame.outer_active && !frame.branch_taken;
                frame.branch_taken |= frame.active;
                return true;
            }
            "endif" | "ifend" => {
                if self.conditionals.pop().is_none() {
                    self.diagnostic(
                        logical_span.clone(),
                        origin,
                        "$endif without matching conditional",
                    );
                }
                return true;
            }
            _ => {}
        }

        if !self.current_active() {
            return is_known_inactive_directive(name);
        }

        match name {
            "define" => {
                let (symbol, value) = split_define(rest);
                if symbol.is_empty() {
                    self.diagnostic(logical_span.clone(), origin, "$define requires a symbol");
                } else {
                    self.defines.insert(canonical_identifier(&symbol), value);
                }
                true
            }
            "undef" => {
                self.defines.remove(&canonical_identifier(rest.trim()));
                true
            }
            "push" => {
                self.saved_states
                    .push((self.state.clone(), origin.clone(), logical_span.clone()));
                true
            }
            "pop" => {
                if let Some((state, _, _)) = self.saved_states.pop() {
                    self.state = state;
                } else {
                    self.diagnostic(logical_span.clone(), origin, "$pop without preceding $push");
                }
                true
            }
            "error" | "fatal" => {
                self.diagnostic(
                    logical_span.clone(),
                    origin,
                    format!("user-defined ${name}: {rest}"),
                );
                true
            }
            "i" | "include" if !looks_like_switch(rest) => {
                self.include(rest, origin, logical_span);
                true
            }
            "interfaces" => {
                match compact_argument(rest).as_str() {
                    "corba" => self.state.interface_model = InterfaceModel::Corba,
                    "com" | "default" => self.state.interface_model = InterfaceModel::Com,
                    _ => self.diagnostic(
                        logical_span.clone(),
                        origin,
                        "$interfaces expects COM, CORBA, or DEFAULT",
                    ),
                }
                true
            }
            "mode" => {
                self.state.language_mode = LanguageMode::parse(rest);
                self.state.mode_switches.clear();
                self.state.macros_enabled = false;
                true
            }
            "modeswitch" => {
                let rest = rest.trim();
                let enabled = !rest.ends_with('-');
                let switch = canonical_identifier(rest.trim_end_matches(['+', '-']));
                if switch.is_empty() {
                    self.diagnostic(
                        logical_span.clone(),
                        origin,
                        "$modeswitch requires a switch name",
                    );
                } else {
                    self.state.mode_switches.insert(switch, enabled);
                }
                true
            }
            "macro" => {
                let compact = compact_argument(rest);
                if let Some(enabled) = parse_on_off(&compact) {
                    self.state.macros_enabled = enabled;
                } else {
                    self.diagnostic(logical_span.clone(), origin, "$macro expects ON or OFF");
                }
                true
            }
            "packrecords" => {
                self.state.record_packing = Some(compact_argument(rest));
                true
            }
            "packenum" => {
                self.state.enum_packing = Some(compact_argument(rest));
                true
            }
            "packset" => {
                self.state.set_packing = Some(compact_argument(rest));
                true
            }
            "iochecks" => {
                self.set_named_switch('i', rest, origin, logical_span);
                true
            }
            "rangechecks" => {
                self.set_named_switch('r', rest, origin, logical_span);
                true
            }
            "overflowchecks" => {
                self.set_named_switch('q', rest, origin, logical_span);
                true
            }
            "typedaddress" => {
                self.set_named_switch('t', rest, origin, logical_span);
                true
            }
            "varstringchecks" => {
                self.set_named_switch('v', rest, origin, logical_span);
                true
            }
            "booleval" => {
                self.set_named_switch('b', rest, origin, logical_span);
                true
            }
            "assertions" => {
                let compact = compact_argument(rest);
                if let Some(enabled) = parse_on_off(&compact) {
                    self.state.assertions = enabled;
                } else {
                    self.diagnostic(
                        logical_span.clone(),
                        origin,
                        "$assertions expects ON or OFF",
                    );
                }
                true
            }
            "warnings" => {
                let compact = compact_argument(rest);
                if let Some(enabled) = parse_on_off(&compact) {
                    self.state.warnings_enabled = enabled;
                } else {
                    self.diagnostic(logical_span.clone(), origin, "$warnings expects ON or OFF");
                }
                true
            }
            "warn" => {
                let mut parts = rest.split_ascii_whitespace();
                let warning = parts.next().unwrap_or_default();
                let setting = parts.next().unwrap_or_default();
                if warning.is_empty() || parse_on_off(&setting.to_ascii_lowercase()).is_none() {
                    self.diagnostic(
                        logical_span.clone(),
                        origin,
                        "$warn expects a warning identifier followed by ON or OFF",
                    );
                } else {
                    self.state.warning_controls.insert(
                        canonical_identifier(warning),
                        parse_on_off(&setting.to_ascii_lowercase()).unwrap(),
                    );
                }
                true
            }
            "asmmode" => {
                self.state.assembler_mode = AssemblerMode::parse(rest);
                true
            }
            "apptype" => {
                self.state.application_type = ApplicationType::parse(rest);
                true
            }
            "note" | "notes" | "inline" | "codepage" | "goto" | "maxfpuregisters"
            | "maxstacksize" | "minstacksize" | "setpeflags" | "implicitexceptions" => true,
            _ if name.len() == 1 && looks_like_switch(rest) => {
                self.set_switch_list(name, rest, origin, logical_span);
                true
            }
            _ => false,
        }
    }

    fn eval_condition(&mut self, expression: &str, origin: &SourceSpan, span: Span) -> bool {
        match evaluate_directive_expression(expression, &self.defines) {
            Ok(value) => value,
            Err(message) => {
                self.diagnostic(span, origin, message);
                false
            }
        }
    }

    fn set_named_switch(
        &mut self,
        letter: char,
        rest: &str,
        origin: &SourceSpan,
        logical_span: &Span,
    ) {
        let compact = compact_argument(rest);
        if let Some(enabled) = parse_on_off(&compact) {
            self.state.set_switch(letter, enabled);
        } else {
            self.diagnostic(logical_span.clone(), origin, "directive expects ON or OFF");
        }
    }

    fn set_switch_list(
        &mut self,
        name: &str,
        rest: &str,
        origin: &SourceSpan,
        logical_span: &Span,
    ) {
        let compact = format!("{name}{}", compact_argument(rest));
        let bytes = compact.as_bytes();
        let mut position = 0;
        while position < bytes.len() {
            if position + 1 >= bytes.len()
                || !bytes[position].is_ascii_alphabetic()
                || !matches!(bytes[position + 1], b'+' | b'-')
            {
                self.diagnostic(logical_span.clone(), origin, "malformed option switch list");
                return;
            }
            self.state
                .set_switch(char::from(bytes[position]), bytes[position + 1] == b'+');
            position += 2;
            if position == bytes.len() {
                return;
            }
            if bytes[position] != b',' {
                self.diagnostic(logical_span.clone(), origin, "malformed option switch list");
                return;
            }
            position += 1;
        }
    }

    fn include(&mut self, rest: &str, origin: &SourceSpan, logical_span: &Span) {
        if self.frames.len() >= self.options.max_include_depth {
            self.diagnostic(
                logical_span.clone(),
                origin,
                "maximum include depth exceeded",
            );
            return;
        }
        let requested = trim_include_name(rest);
        if requested.starts_with('%') && requested.ends_with('%') {
            if !requested.eq_ignore_ascii_case("%date%") {
                self.diagnostic(
                    logical_span.clone(),
                    origin,
                    format!("unsupported include macro `{requested}`"),
                );
                return;
            }
            let text = format!("'{}'", self.options.date_macro.replace('\'', "''"));
            let source = self.add_source(
                format!("<{requested}>"),
                text,
                None,
                Some(origin.clone()),
                true,
            );
            self.dependencies.push(IncludeDependency {
                directive: origin.clone(),
                included: source,
            });
            self.push_source(source);
            return;
        }

        let Some(path) = self.resolve_include_path(&requested) else {
            self.diagnostic(
                logical_span.clone(),
                origin,
                format!("cannot open include file `{requested}`"),
            );
            return;
        };
        if self.active_paths.contains(&path) {
            self.diagnostic(
                logical_span.clone(),
                origin,
                format!("include cycle through `{}`", path.display()),
            );
            return;
        }
        let source = if let Some(source) = self.sources_by_path.get(&path).copied() {
            source
        } else {
            let text = match fs::read_to_string(&path) {
                Ok(text) => text,
                Err(error) => {
                    self.diagnostic(
                        logical_span.clone(),
                        origin,
                        format!("cannot read include file `{}`: {error}", path.display()),
                    );
                    return;
                }
            };
            self.add_source(
                path.to_string_lossy().into_owned(),
                text,
                Some(path.clone()),
                Some(origin.clone()),
                false,
            )
        };
        self.dependencies.push(IncludeDependency {
            directive: origin.clone(),
            included: source,
        });
        self.push_source(source);
    }

    fn expand_macro(&mut self, name: &str, origin: &SourceSpan, logical_span: &Span) -> bool {
        let name = canonical_identifier(name);
        if self.active_macros.contains(&name) {
            self.diagnostic(
                logical_span.clone(),
                origin,
                format!("recursive source macro expansion of `{name}`"),
            );
            return false;
        }
        if self.active_macros.len() >= self.options.max_macro_expansion_depth {
            self.diagnostic(
                logical_span.clone(),
                origin,
                "maximum source macro expansion depth exceeded",
            );
            return false;
        }
        let Some(replacement) = self.defines.get(&name).cloned() else {
            return false;
        };
        let source = self.add_source(
            format!("<macro {name}>"),
            replacement,
            None,
            Some(origin.clone()),
            true,
        );
        self.macro_expansions.push(MacroExpansion {
            name: name.clone(),
            invocation: origin.clone(),
            expanded_source: source,
        });
        self.push_source_with_macro(source, Some(name));
        true
    }

    fn resolve_include_path(&self, requested: &str) -> Option<PathBuf> {
        let requested = Path::new(requested);
        let mut candidates = Vec::new();
        if requested.is_absolute() {
            candidates.push(requested.to_path_buf());
        } else {
            if let Some(current) = self.frames.last()
                && let Some(path) = self.buffers[current.source.as_u32() as usize].path.as_ref()
                && let Some(parent) = path.parent()
            {
                candidates.push(parent.join(requested));
            }
            candidates.extend(
                self.options
                    .include_paths
                    .iter()
                    .map(|directory| directory.join(requested)),
            );
            candidates.push(requested.to_path_buf());
        }
        candidates.into_iter().find_map(|candidate| {
            candidate
                .is_file()
                .then(|| fs::canonicalize(&candidate).unwrap_or(candidate))
        })
    }
}

pub fn preprocess(source_name: &str, source: &str, options: &PreprocessorOptions) -> LexOutput {
    let mut session = PreprocessorSession::new(options);
    let path = source_path(source_name);
    let root = session.add_source(source_name.to_owned(), source.to_owned(), path, None, false);
    session.push_source(root);
    session.run()
}

fn source_path(source_name: &str) -> Option<PathBuf> {
    if source_name.starts_with('<') && source_name.ends_with('>') {
        return None;
    }
    let path = PathBuf::from(source_name);
    Some(fs::canonicalize(&path).unwrap_or(path))
}

fn canonical_identifier(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

fn split_directive(body: &str) -> (String, String) {
    let body = body.trim();
    let name_end = body
        .char_indices()
        .find_map(|(index, character)| {
            (!character.is_ascii_alphanumeric() && character != '_').then_some(index)
        })
        .unwrap_or(body.len());
    (
        canonical_identifier(&body[..name_end]),
        body[name_end..].trim().to_owned(),
    )
}

fn compact_argument(argument: &str) -> String {
    argument
        .chars()
        .filter(|character| !character.is_ascii_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

fn split_define(rest: &str) -> (String, String) {
    if let Some((symbol, value)) = rest.split_once(":=") {
        (symbol.trim().to_owned(), value.trim().to_owned())
    } else {
        (rest.trim().to_owned(), String::new())
    }
}

fn looks_like_switch(rest: &str) -> bool {
    compact_argument(rest)
        .as_bytes()
        .first()
        .is_some_and(|byte| matches!(byte, b'+' | b'-'))
}

fn parse_on_off(compact: &str) -> Option<bool> {
    match compact {
        "on" | "+" => Some(true),
        "off" | "-" => Some(false),
        _ => None,
    }
}

fn trim_include_name(rest: &str) -> String {
    let rest = rest.trim();
    if rest.len() >= 2
        && ((rest.starts_with('\'') && rest.ends_with('\''))
            || (rest.starts_with('"') && rest.ends_with('"')))
    {
        rest[1..rest.len() - 1].to_owned()
    } else {
        rest.to_owned()
    }
}

fn is_known_inactive_directive(name: &str) -> bool {
    matches!(
        name,
        "define"
            | "undef"
            | "push"
            | "pop"
            | "error"
            | "fatal"
            | "i"
            | "include"
            | "interfaces"
            | "mode"
            | "modeswitch"
            | "macro"
            | "packrecords"
            | "packenum"
            | "packset"
            | "iochecks"
            | "rangechecks"
            | "overflowchecks"
            | "typedaddress"
            | "varstringchecks"
            | "booleval"
            | "assertions"
            | "warn"
            | "warnings"
            | "note"
            | "notes"
            | "inline"
            | "asmmode"
            | "apptype"
            | "codepage"
            | "goto"
            | "maxfpuregisters"
            | "maxstacksize"
            | "minstacksize"
            | "setpeflags"
            | "implicitexceptions"
    ) || name.len() == 1
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum DirectiveValue {
    Integer(i128),
    Boolean(bool),
    String(String),
}

impl DirectiveValue {
    fn boolean(&self) -> Result<bool, String> {
        match self {
            Self::Boolean(value) => Ok(*value),
            Self::Integer(value) => Ok(*value != 0),
            Self::String(value) => match value.to_ascii_lowercase().as_str() {
                "true" => Ok(true),
                "false" => Ok(false),
                _ => Err(format!("`{value}` is not a Boolean directive value")),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum DirectiveExprToken {
    Identifier(String),
    Integer(i128),
    String(String),
    LeftParen,
    RightParen,
    Plus,
    Minus,
    Star,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    End,
}

struct DirectiveExpression<'a> {
    tokens: Vec<DirectiveExprToken>,
    position: usize,
    defines: &'a BTreeMap<String, String>,
}

impl<'a> DirectiveExpression<'a> {
    fn parse(
        expression: &str,
        defines: &'a BTreeMap<String, String>,
    ) -> Result<DirectiveValue, String> {
        let mut parser = Self {
            tokens: tokenize_directive_expression(expression)?,
            position: 0,
            defines,
        };
        let value = parser.boolean_or()?;
        if parser.peek() != &DirectiveExprToken::End {
            return Err("unexpected token in conditional expression".to_owned());
        }
        Ok(value)
    }

    fn boolean_or(&mut self) -> Result<DirectiveValue, String> {
        let mut value = self.boolean_and()?;
        loop {
            let operator = match self.peek() {
                DirectiveExprToken::Identifier(operator)
                    if matches!(operator.as_str(), "or" | "xor") =>
                {
                    operator.clone()
                }
                _ => return Ok(value),
            };
            self.position += 1;
            let right = self.boolean_and()?;
            value = match operator.as_str() {
                "or" => DirectiveValue::Boolean(value.boolean()? || right.boolean()?),
                "xor" => DirectiveValue::Boolean(value.boolean()? ^ right.boolean()?),
                _ => unreachable!(),
            };
        }
    }

    fn boolean_and(&mut self) -> Result<DirectiveValue, String> {
        let mut value = self.relation()?;
        loop {
            let DirectiveExprToken::Identifier(operator) = self.peek() else {
                return Ok(value);
            };
            if operator != "and" {
                return Ok(value);
            }
            self.position += 1;
            let right = self.relation()?;
            value = DirectiveValue::Boolean(value.boolean()? && right.boolean()?);
        }
    }

    fn relation(&mut self) -> Result<DirectiveValue, String> {
        let left = self.additive()?;
        let operator = self.peek().clone();
        if !matches!(
            operator,
            DirectiveExprToken::Equal
                | DirectiveExprToken::NotEqual
                | DirectiveExprToken::Less
                | DirectiveExprToken::LessEqual
                | DirectiveExprToken::Greater
                | DirectiveExprToken::GreaterEqual
        ) {
            return Ok(left);
        }
        self.position += 1;
        let right = self.additive()?;
        let ordering = compare_directive_values(&left, &right)?;
        let value = match operator {
            DirectiveExprToken::Equal => ordering == std::cmp::Ordering::Equal,
            DirectiveExprToken::NotEqual => ordering != std::cmp::Ordering::Equal,
            DirectiveExprToken::Less => ordering == std::cmp::Ordering::Less,
            DirectiveExprToken::LessEqual => ordering != std::cmp::Ordering::Greater,
            DirectiveExprToken::Greater => ordering == std::cmp::Ordering::Greater,
            DirectiveExprToken::GreaterEqual => ordering != std::cmp::Ordering::Less,
            _ => unreachable!(),
        };
        Ok(DirectiveValue::Boolean(value))
    }

    fn additive(&mut self) -> Result<DirectiveValue, String> {
        let mut value = self.term()?;
        loop {
            let operator = match self.peek() {
                DirectiveExprToken::Plus => "+".to_owned(),
                DirectiveExprToken::Minus => "-".to_owned(),
                _ => return Ok(value),
            };
            self.position += 1;
            let right = self.term()?;
            value = match operator.as_str() {
                "+" => DirectiveValue::Integer(integer_value(value)? + integer_value(right)?),
                "-" => DirectiveValue::Integer(integer_value(value)? - integer_value(right)?),
                _ => unreachable!(),
            };
        }
    }

    fn term(&mut self) -> Result<DirectiveValue, String> {
        let mut value = self.unary()?;
        loop {
            let operator = match self.peek() {
                DirectiveExprToken::Identifier(operator)
                    if matches!(operator.as_str(), "div" | "mod") =>
                {
                    operator.clone()
                }
                DirectiveExprToken::Star => "*".to_owned(),
                _ => return Ok(value),
            };
            self.position += 1;
            let right = self.unary()?;
            value = match operator.as_str() {
                "*" => DirectiveValue::Integer(integer_value(value)? * integer_value(right)?),
                "div" => {
                    let divisor = integer_value(right)?;
                    if divisor == 0 {
                        return Err("division by zero in conditional expression".to_owned());
                    }
                    DirectiveValue::Integer(integer_value(value)? / divisor)
                }
                "mod" => {
                    let divisor = integer_value(right)?;
                    if divisor == 0 {
                        return Err("division by zero in conditional expression".to_owned());
                    }
                    DirectiveValue::Integer(integer_value(value)? % divisor)
                }
                _ => unreachable!(),
            };
        }
    }

    fn unary(&mut self) -> Result<DirectiveValue, String> {
        match self.peek() {
            DirectiveExprToken::Identifier(operator) if operator == "not" => {
                self.position += 1;
                Ok(DirectiveValue::Boolean(!self.unary()?.boolean()?))
            }
            DirectiveExprToken::Plus => {
                self.position += 1;
                Ok(DirectiveValue::Integer(integer_value(self.unary()?)?))
            }
            DirectiveExprToken::Minus => {
                self.position += 1;
                Ok(DirectiveValue::Integer(-integer_value(self.unary()?)?))
            }
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Result<DirectiveValue, String> {
        match self.take() {
            DirectiveExprToken::Integer(value) => Ok(DirectiveValue::Integer(value)),
            DirectiveExprToken::String(value) => Ok(DirectiveValue::String(value)),
            DirectiveExprToken::Identifier(name) if name == "true" => {
                Ok(DirectiveValue::Boolean(true))
            }
            DirectiveExprToken::Identifier(name) if name == "false" => {
                Ok(DirectiveValue::Boolean(false))
            }
            DirectiveExprToken::Identifier(name) if name == "defined" => {
                let parenthesized = self.peek() == &DirectiveExprToken::LeftParen;
                if parenthesized {
                    self.position += 1;
                }
                let DirectiveExprToken::Identifier(symbol) = self.take() else {
                    return Err("defined expects an identifier".to_owned());
                };
                if parenthesized {
                    self.expect(DirectiveExprToken::RightParen)?;
                }
                Ok(DirectiveValue::Boolean(self.defines.contains_key(&symbol)))
            }
            DirectiveExprToken::Identifier(name) => {
                let Some(value) = self.defines.get(&name) else {
                    return Err(format!("undefined directive symbol `{name}`"));
                };
                parse_define_value(value)
            }
            DirectiveExprToken::LeftParen => {
                let value = self.boolean_or()?;
                self.expect(DirectiveExprToken::RightParen)?;
                Ok(value)
            }
            token => Err(format!(
                "unexpected token `{token:?}` in conditional expression"
            )),
        }
    }

    fn peek(&self) -> &DirectiveExprToken {
        self.tokens
            .get(self.position)
            .unwrap_or(&DirectiveExprToken::End)
    }

    fn take(&mut self) -> DirectiveExprToken {
        let token = self.peek().clone();
        self.position = self.position.saturating_add(1);
        token
    }

    fn expect(&mut self, expected: DirectiveExprToken) -> Result<(), String> {
        let actual = self.take();
        if actual == expected {
            Ok(())
        } else {
            Err(format!("expected `{expected:?}`, found `{actual:?}`"))
        }
    }
}

fn evaluate_directive_expression(
    expression: &str,
    defines: &BTreeMap<String, String>,
) -> Result<bool, String> {
    DirectiveExpression::parse(expression, defines)?.boolean()
}

fn integer_value(value: DirectiveValue) -> Result<i128, String> {
    match value {
        DirectiveValue::Integer(value) => Ok(value),
        other => Err(format!(
            "expected integer directive value, found `{other:?}`"
        )),
    }
}

fn compare_directive_values(
    left: &DirectiveValue,
    right: &DirectiveValue,
) -> Result<std::cmp::Ordering, String> {
    match (left, right) {
        (DirectiveValue::Integer(left), DirectiveValue::Integer(right)) => Ok(left.cmp(right)),
        (DirectiveValue::Boolean(left), DirectiveValue::Boolean(right)) => Ok(left.cmp(right)),
        (DirectiveValue::String(left), DirectiveValue::String(right)) => Ok(left.cmp(right)),
        _ => Err("conditional comparison requires values of one kind".to_owned()),
    }
}

fn parse_define_value(value: &str) -> Result<DirectiveValue, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(DirectiveValue::Boolean(true));
    }
    if value.eq_ignore_ascii_case("true") {
        return Ok(DirectiveValue::Boolean(true));
    }
    if value.eq_ignore_ascii_case("false") {
        return Ok(DirectiveValue::Boolean(false));
    }
    if let Some(value) = value.strip_prefix('$') {
        return i128::from_str_radix(value, 16)
            .map(DirectiveValue::Integer)
            .map_err(|_| "invalid hexadecimal define value".to_owned());
    }
    if let Some(value) = value.strip_prefix('%') {
        return i128::from_str_radix(value, 2)
            .map(DirectiveValue::Integer)
            .map_err(|_| "invalid binary define value".to_owned());
    }
    if let Ok(value) = value.parse::<i128>() {
        return Ok(DirectiveValue::Integer(value));
    }
    if value.len() >= 2 && value.starts_with('\'') && value.ends_with('\'') {
        return Ok(DirectiveValue::String(
            value[1..value.len() - 1].replace("''", "'"),
        ));
    }
    Ok(DirectiveValue::String(value.to_owned()))
}

fn tokenize_directive_expression(expression: &str) -> Result<Vec<DirectiveExprToken>, String> {
    let bytes = expression.as_bytes();
    let mut position = 0;
    let mut tokens = Vec::new();
    while position < bytes.len() {
        let byte = bytes[position];
        if byte.is_ascii_whitespace() {
            position += 1;
            continue;
        }
        if byte.is_ascii_alphabetic() || byte == b'_' {
            let start = position;
            position += 1;
            while position < bytes.len()
                && (bytes[position].is_ascii_alphanumeric() || bytes[position] == b'_')
            {
                position += 1;
            }
            tokens.push(DirectiveExprToken::Identifier(
                expression[start..position].to_ascii_lowercase(),
            ));
            continue;
        }
        if byte.is_ascii_digit() {
            let start = position;
            position += 1;
            while position < bytes.len()
                && (bytes[position].is_ascii_digit() || bytes[position] == b'_')
            {
                position += 1;
            }
            let digits = expression[start..position].replace('_', "");
            tokens.push(DirectiveExprToken::Integer(digits.parse().map_err(
                |_| "invalid integer in conditional expression".to_owned(),
            )?));
            continue;
        }
        if matches!(byte, b'$' | b'%') {
            let radix = if byte == b'$' { 16 } else { 2 };
            position += 1;
            let start = position;
            while position < bytes.len()
                && (bytes[position].is_ascii_hexdigit() || bytes[position] == b'_')
            {
                position += 1;
            }
            let digits = expression[start..position].replace('_', "");
            tokens.push(DirectiveExprToken::Integer(
                i128::from_str_radix(&digits, radix)
                    .map_err(|_| "invalid based integer in conditional expression".to_owned())?,
            ));
            continue;
        }
        if byte == b'\'' {
            position += 1;
            let mut value = String::new();
            loop {
                if position >= bytes.len() {
                    return Err("unterminated string in conditional expression".to_owned());
                }
                if bytes[position] == b'\'' {
                    position += 1;
                    if position < bytes.len() && bytes[position] == b'\'' {
                        value.push('\'');
                        position += 1;
                        continue;
                    }
                    break;
                }
                value.push(char::from(bytes[position]));
                position += 1;
            }
            tokens.push(DirectiveExprToken::String(value));
            continue;
        }
        let (token, width) = match &expression[position..] {
            rest if rest.starts_with("<=") => (DirectiveExprToken::LessEqual, 2),
            rest if rest.starts_with(">=") => (DirectiveExprToken::GreaterEqual, 2),
            rest if rest.starts_with("<>") => (DirectiveExprToken::NotEqual, 2),
            _ => match byte {
                b'(' => (DirectiveExprToken::LeftParen, 1),
                b')' => (DirectiveExprToken::RightParen, 1),
                b'+' => (DirectiveExprToken::Plus, 1),
                b'-' => (DirectiveExprToken::Minus, 1),
                b'*' => (DirectiveExprToken::Star, 1),
                b'=' => (DirectiveExprToken::Equal, 1),
                b'<' => (DirectiveExprToken::Less, 1),
                b'>' => (DirectiveExprToken::Greater, 1),
                _ => {
                    return Err(format!(
                        "invalid character `{}` in conditional expression",
                        char::from(byte)
                    ));
                }
            },
        };
        tokens.push(token);
        position += width;
    }
    tokens.push(DirectiveExprToken::End);
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directive_expression_supports_defines_boolean_logic_and_numeric_comparisons() {
        let mut defines = BTreeMap::new();
        defines.insert("unix".to_owned(), String::new());
        defines.insert("fpc_fullversion".to_owned(), "30200".to_owned());
        assert!(
            evaluate_directive_expression("defined(unix) and FPC_FULLVERSION >= 30200", &defines,)
                .unwrap()
        );
        assert!(
            !evaluate_directive_expression("defined(win32) or FPC_FULLVERSION < 20600", &defines,)
                .unwrap()
        );
    }
}
