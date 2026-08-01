use std::collections::{BTreeMap, BTreeSet};

use super::ids::{
    EnvironmentId, ModuleId, NameId, ReceiverId, RegionId, SymbolId, TypeRef, TypeSectionId,
};

#[derive(Clone, Debug, Default)]
pub struct NameInterner {
    names: Vec<String>,
    ids: BTreeMap<String, NameId>,
}

impl NameInterner {
    pub fn intern(&mut self, spelling: &str) -> NameId {
        let canonical = spelling.to_ascii_lowercase();
        if let Some(id) = self.ids.get(&canonical) {
            return *id;
        }
        let id = NameId::from_index(self.names.len());
        self.names.push(canonical.clone());
        self.ids.insert(canonical, id);
        id
    }

    pub fn spelling(&self, name: NameId) -> &str {
        &self.names[name.index()]
    }

    pub fn lookup(&self, spelling: &str) -> Option<NameId> {
        self.ids.get(&spelling.to_ascii_lowercase()).copied()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolCategory {
    Type,
    Routine,
    Constant,
    Variable,
    Property,
    Label,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolFilter {
    Any,
    Type,
    Value,
    Category(SymbolCategory),
}

impl SymbolFilter {
    pub fn accepts(self, category: SymbolCategory) -> bool {
        match self {
            Self::Any => true,
            Self::Type => category == SymbolCategory::Type,
            Self::Value => matches!(
                category,
                SymbolCategory::Routine
                    | SymbolCategory::Constant
                    | SymbolCategory::Variable
                    | SymbolCategory::Property
            ),
            Self::Category(expected) => category == expected,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    Type(TypeRef),
    Routine(TypeRef),
    Constant(TypeRef),
    Variable(TypeRef),
    Property(TypeRef),
    Label,
}

impl SymbolKind {
    pub const fn category(&self) -> SymbolCategory {
        match self {
            Self::Type(_) => SymbolCategory::Type,
            Self::Routine(_) => SymbolCategory::Routine,
            Self::Constant(_) => SymbolCategory::Constant,
            Self::Variable(_) => SymbolCategory::Variable,
            Self::Property(_) => SymbolCategory::Property,
            Self::Label => SymbolCategory::Label,
        }
    }

    pub const fn ty(&self) -> Option<TypeRef> {
        match self {
            Self::Type(ty)
            | Self::Routine(ty)
            | Self::Constant(ty)
            | Self::Variable(ty)
            | Self::Property(ty) => Some(*ty),
            Self::Label => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeclarationState {
    Defining,
    Complete,
    Forward,
    Error,
}

#[derive(Clone, Debug)]
pub struct Symbol {
    pub name: NameId,
    pub kind: SymbolKind,
    pub state: DeclarationState,
    pub declared_in: EnvironmentId,
    pub region: RegionId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RegionOwner {
    Root,
    Module(ModuleId),
    Routine(TypeRef),
    Type(TypeRef),
    Block(u32),
}

#[derive(Clone, Debug)]
struct Region {
    owner: RegionOwner,
    declarations: BTreeMap<NameId, Vec<SymbolId>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameKind {
    RegionEntry,
    ConstSection,
    TypeSection(TypeSectionId),
    VarSection,
    InlineDeclaration,
    CompoundBlock,
    LookupOverlay,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookupEdgeKind {
    LexicalParent,
    InheritedMembers,
    ImplicitSelf,
    WithReceiver,
    ModuleImport,
    System,
}

#[derive(Clone, Debug)]
pub struct LookupEdge {
    pub target: EnvironmentId,
    pub kind: LookupEdgeKind,
    pub receiver: Option<ReceiverId>,
    pub imported_from: Option<ModuleId>,
}

impl LookupEdge {
    pub const fn lexical_parent(target: EnvironmentId) -> Self {
        Self {
            target,
            kind: LookupEdgeKind::LexicalParent,
            receiver: None,
            imported_from: None,
        }
    }

    pub const fn import(target: EnvironmentId, module: ModuleId) -> Self {
        Self {
            target,
            kind: LookupEdgeKind::ModuleImport,
            receiver: None,
            imported_from: Some(module),
        }
    }

    pub const fn system(target: EnvironmentId, module: ModuleId) -> Self {
        Self {
            target,
            kind: LookupEdgeKind::System,
            receiver: None,
            imported_from: Some(module),
        }
    }

    pub const fn inherited_members(target: EnvironmentId) -> Self {
        Self {
            target,
            kind: LookupEdgeKind::InheritedMembers,
            receiver: None,
            imported_from: None,
        }
    }

    pub const fn implicit_self(target: EnvironmentId, receiver: ReceiverId) -> Self {
        Self::receiver(target, LookupEdgeKind::ImplicitSelf, receiver)
    }

    pub const fn with_receiver(target: EnvironmentId, receiver: ReceiverId) -> Self {
        Self::receiver(target, LookupEdgeKind::WithReceiver, receiver)
    }

    pub const fn receiver(
        target: EnvironmentId,
        kind: LookupEdgeKind,
        receiver: ReceiverId,
    ) -> Self {
        Self {
            target,
            kind,
            receiver: Some(receiver),
            imported_from: None,
        }
    }
}

#[derive(Clone, Debug)]
struct Environment {
    region: RegionId,
    kind: FrameKind,
    symbols: BTreeMap<NameId, Vec<SymbolId>>,
    fallbacks: Vec<LookupEdge>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LookupBarrier {
    AnyDeclaration,
    AcceptedDeclaration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LookupRequest {
    pub accepted: SymbolFilter,
    pub barrier: LookupBarrier,
}

impl LookupRequest {
    pub const ORDINARY: Self = Self {
        accepted: SymbolFilter::Any,
        barrier: LookupBarrier::AnyDeclaration,
    };

    pub const REQUIRED_TYPE: Self = Self {
        accepted: SymbolFilter::Type,
        barrier: LookupBarrier::AcceptedDeclaration,
    };

    pub const REQUIRED_VALUE: Self = Self {
        accepted: SymbolFilter::Value,
        barrier: LookupBarrier::AcceptedDeclaration,
    };
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LookupStep {
    pub from: EnvironmentId,
    pub to: EnvironmentId,
    pub kind: LookupEdgeKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LookupHit {
    pub symbol: SymbolId,
    pub declaring_environment: EnvironmentId,
    pub receiver: Option<ReceiverId>,
    pub imported_from: Option<ModuleId>,
    pub path: Vec<LookupStep>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LookupResult {
    pub primary: Vec<LookupHit>,
    pub shadowed: Vec<Vec<LookupHit>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnvironmentCheckpoint {
    previous: EnvironmentId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeclarationMode {
    Fresh,
    Overload,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeclareError {
    Duplicate {
        name: NameId,
        existing: Vec<SymbolId>,
    },
    NonRoutineOverload {
        name: NameId,
        existing: Vec<SymbolId>,
    },
}

#[derive(Clone, Debug)]
struct FoundLayer {
    symbols: Vec<SymbolId>,
    environment: EnvironmentId,
    receiver: Option<ReceiverId>,
    imported_from: Option<ModuleId>,
    path: Vec<LookupStep>,
}

#[derive(Clone, Debug)]
pub struct ScopeGraph {
    names: NameInterner,
    regions: Vec<Region>,
    environments: Vec<Environment>,
    symbols: Vec<Symbol>,
    current: EnvironmentId,
}

impl Default for ScopeGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl ScopeGraph {
    pub fn new() -> Self {
        let root_region = RegionId::from_index(0);
        let root_environment = EnvironmentId::from_index(0);
        Self {
            names: NameInterner::default(),
            regions: vec![Region {
                owner: RegionOwner::Root,
                declarations: BTreeMap::new(),
            }],
            environments: vec![Environment {
                region: root_region,
                kind: FrameKind::RegionEntry,
                symbols: BTreeMap::new(),
                fallbacks: Vec::new(),
            }],
            symbols: Vec::new(),
            current: root_environment,
        }
    }

    pub fn names(&self) -> &NameInterner {
        &self.names
    }

    pub fn intern_name(&mut self, spelling: &str) -> NameId {
        self.names.intern(spelling)
    }

    pub const fn current_environment(&self) -> EnvironmentId {
        self.current
    }

    pub fn select_environment(&mut self, environment: EnvironmentId) -> EnvironmentCheckpoint {
        let previous = self.current;
        self.current = environment;
        EnvironmentCheckpoint { previous }
    }

    pub fn environment_region(&self, environment: EnvironmentId) -> RegionId {
        self.environments[environment.index()].region
    }

    pub fn environment_kind(&self, environment: EnvironmentId) -> FrameKind {
        self.environments[environment.index()].kind
    }

    pub fn region_owner(&self, region: RegionId) -> RegionOwner {
        self.regions[region.index()].owner
    }

    pub fn symbol(&self, symbol: SymbolId) -> &Symbol {
        &self.symbols[symbol.index()]
    }

    pub fn create_detached_region(
        &mut self,
        owner: RegionOwner,
        fallbacks: Vec<LookupEdge>,
    ) -> (RegionId, EnvironmentId) {
        let region = RegionId::from_index(self.regions.len());
        self.regions.push(Region {
            owner,
            declarations: BTreeMap::new(),
        });
        let environment = self.allocate_environment(region, FrameKind::RegionEntry, fallbacks);
        (region, environment)
    }

    pub fn enter_region(&mut self, owner: RegionOwner) -> (RegionId, EnvironmentCheckpoint) {
        let previous = self.current;
        let (region, environment) =
            self.create_detached_region(owner, vec![LookupEdge::lexical_parent(previous)]);
        self.current = environment;
        (region, EnvironmentCheckpoint { previous })
    }

    pub fn exit_region(&mut self, checkpoint: EnvironmentCheckpoint) {
        self.current = checkpoint.previous;
    }

    pub fn extend_environment(&mut self, kind: FrameKind) -> EnvironmentId {
        let region = self.environment_region(self.current);
        let environment =
            self.allocate_environment(region, kind, vec![LookupEdge::lexical_parent(self.current)]);
        self.current = environment;
        environment
    }

    pub fn push_overlay(
        &mut self,
        high_to_low_precedence: Vec<LookupEdge>,
    ) -> EnvironmentCheckpoint {
        let previous = self.current;
        let region = self.environment_region(previous);
        let mut fallbacks = high_to_low_precedence;
        fallbacks.push(LookupEdge::lexical_parent(previous));
        self.current = self.create_lookup_environment(region, fallbacks);
        EnvironmentCheckpoint { previous }
    }

    pub fn create_lookup_environment(
        &mut self,
        region: RegionId,
        high_to_low_precedence: Vec<LookupEdge>,
    ) -> EnvironmentId {
        self.allocate_environment(region, FrameKind::LookupOverlay, high_to_low_precedence)
    }

    /// Builds a stable view containing only declarations owned by `region`.
    /// Its optional fallbacks are explicit, so unit imports and outer lexical
    /// scopes cannot leak through an exported unit/member environment.
    pub fn create_region_view(
        &mut self,
        region: RegionId,
        fallbacks: Vec<LookupEdge>,
    ) -> EnvironmentId {
        let environment = self.allocate_environment(region, FrameKind::LookupOverlay, fallbacks);
        self.environments[environment.index()].symbols =
            self.regions[region.index()].declarations.clone();
        environment
    }

    pub fn restore_environment(&mut self, checkpoint: EnvironmentCheckpoint) {
        self.current = checkpoint.previous;
    }

    pub fn declare(
        &mut self,
        name: NameId,
        kind: SymbolKind,
        state: DeclarationState,
        mode: DeclarationMode,
    ) -> Result<SymbolId, DeclareError> {
        let previous = self.current;
        let region = self.environment_region(previous);
        let existing = self.regions[region.index()]
            .declarations
            .get(&name)
            .cloned()
            .unwrap_or_default();

        match mode {
            DeclarationMode::Fresh if !existing.is_empty() => {
                return Err(DeclareError::Duplicate { name, existing });
            }
            DeclarationMode::Overload
                if existing.iter().any(|symbol| {
                    self.symbols[symbol.index()].kind.category() != SymbolCategory::Routine
                }) =>
            {
                return Err(DeclareError::NonRoutineOverload { name, existing });
            }
            DeclarationMode::Fresh | DeclarationMode::Overload => {}
        }

        // Each successful declaration advances the persistent environment.
        // A nested routine can therefore retain the exact environment visible
        // at its declaration without later declarations leaking backwards.
        let environment = self.allocate_environment(
            region,
            self.environment_kind(previous),
            vec![LookupEdge::lexical_parent(previous)],
        );
        self.current = environment;

        let symbol = SymbolId::from_index(self.symbols.len());
        self.symbols.push(Symbol {
            name,
            kind,
            state,
            declared_in: environment,
            region,
        });
        self.regions[region.index()]
            .declarations
            .entry(name)
            .or_default()
            .push(symbol);

        let bucket = self.environments[environment.index()]
            .symbols
            .entry(name)
            .or_default();
        if mode == DeclarationMode::Overload {
            bucket.extend(existing);
            bucket.sort();
            bucket.dedup();
        }
        bucket.push(symbol);
        Ok(symbol)
    }

    pub fn complete_symbol(&mut self, symbol: SymbolId, kind: SymbolKind) {
        let symbol = &mut self.symbols[symbol.index()];
        symbol.kind = kind;
        symbol.state = DeclarationState::Complete;
    }

    pub fn set_symbol_state(&mut self, symbol: SymbolId, state: DeclarationState) {
        self.symbols[symbol.index()].state = state;
    }

    pub fn lookup_symbol(
        &self,
        start: EnvironmentId,
        name: NameId,
        request: LookupRequest,
    ) -> Option<LookupResult> {
        let mut layers = Vec::new();
        self.collect_layers(
            start,
            name,
            &mut BTreeSet::new(),
            Vec::new(),
            None,
            None,
            &mut layers,
        );

        let primary_index = layers.iter().position(|layer| {
            request.barrier == LookupBarrier::AnyDeclaration
                || layer.symbols.iter().any(|symbol| {
                    request
                        .accepted
                        .accepts(self.symbols[symbol.index()].kind.category())
                })
        })?;

        let to_hits = |layer: &FoundLayer, filter: bool| {
            layer
                .symbols
                .iter()
                .filter(|symbol| {
                    !filter
                        || request
                            .accepted
                            .accepts(self.symbols[symbol.index()].kind.category())
                })
                .map(|symbol| LookupHit {
                    symbol: *symbol,
                    declaring_environment: layer.environment,
                    receiver: layer.receiver,
                    imported_from: layer.imported_from,
                    path: layer.path.clone(),
                })
                .collect::<Vec<_>>()
        };

        let primary = to_hits(
            &layers[primary_index],
            request.barrier == LookupBarrier::AcceptedDeclaration,
        );
        let shadowed = layers[primary_index + 1..]
            .iter()
            .map(|layer| to_hits(layer, false))
            .filter(|layer| !layer.is_empty())
            .collect();
        Some(LookupResult { primary, shadowed })
    }

    fn allocate_environment(
        &mut self,
        region: RegionId,
        kind: FrameKind,
        fallbacks: Vec<LookupEdge>,
    ) -> EnvironmentId {
        let id = EnvironmentId::from_index(self.environments.len());
        self.environments.push(Environment {
            region,
            kind,
            symbols: BTreeMap::new(),
            fallbacks,
        });
        id
    }

    #[allow(clippy::too_many_arguments)]
    fn collect_layers(
        &self,
        environment: EnvironmentId,
        name: NameId,
        visited: &mut BTreeSet<EnvironmentId>,
        path: Vec<LookupStep>,
        receiver: Option<ReceiverId>,
        imported_from: Option<ModuleId>,
        output: &mut Vec<FoundLayer>,
    ) {
        if !visited.insert(environment) {
            return;
        }
        let frame = &self.environments[environment.index()];
        if let Some(symbols) = frame.symbols.get(&name) {
            output.push(FoundLayer {
                symbols: symbols.clone(),
                environment,
                receiver,
                imported_from,
                path: path.clone(),
            });
        }
        for edge in &frame.fallbacks {
            let mut child_path = path.clone();
            child_path.push(LookupStep {
                from: environment,
                to: edge.target,
                kind: edge.kind,
            });
            self.collect_layers(
                edge.target,
                name,
                visited,
                child_path,
                edge.receiver.or(receiver),
                edge.imported_from.or(imported_from),
                output,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_type(index: u32) -> TypeRef {
        TypeRef(index)
    }

    #[test]
    fn persistent_frames_preserve_declaration_point_visibility() {
        let mut graph = ScopeGraph::new();
        let (_, checkpoint) = graph.enter_region(RegionOwner::Routine(fake_type(0)));
        graph.extend_environment(FrameKind::VarSection);
        let before = graph.intern_name("Before");
        graph
            .declare(
                before,
                SymbolKind::Variable(fake_type(1)),
                DeclarationState::Complete,
                DeclarationMode::Fresh,
            )
            .unwrap();

        graph.extend_environment(FrameKind::InlineDeclaration);
        let inner = graph.intern_name("Inner");
        graph
            .declare(
                inner,
                SymbolKind::Routine(fake_type(2)),
                DeclarationState::Complete,
                DeclarationMode::Fresh,
            )
            .unwrap();
        let inner_parent = graph.current_environment();

        graph.extend_environment(FrameKind::VarSection);
        let after = graph.intern_name("After");
        graph
            .declare(
                after,
                SymbolKind::Variable(fake_type(1)),
                DeclarationState::Complete,
                DeclarationMode::Fresh,
            )
            .unwrap();

        assert!(
            graph
                .lookup_symbol(inner_parent, before, LookupRequest::ORDINARY)
                .is_some()
        );
        assert!(
            graph
                .lookup_symbol(inner_parent, inner, LookupRequest::ORDINARY)
                .is_some()
        );
        assert!(
            graph
                .lookup_symbol(inner_parent, after, LookupRequest::ORDINARY)
                .is_none()
        );
        graph.exit_region(checkpoint);
    }

    #[test]
    fn later_import_wins_but_required_type_can_skip_its_value() {
        let mut graph = ScopeGraph::new();
        let name = graph.intern_name("X");

        let (_, mut a) = graph.create_detached_region(RegionOwner::Module(ModuleId(0)), Vec::new());
        graph.current = a;
        let a_type = graph
            .declare(
                name,
                SymbolKind::Type(fake_type(10)),
                DeclarationState::Complete,
                DeclarationMode::Fresh,
            )
            .unwrap();
        a = graph.current_environment();

        let (_, mut b) = graph.create_detached_region(RegionOwner::Module(ModuleId(1)), Vec::new());
        graph.current = b;
        let b_value = graph
            .declare(
                name,
                SymbolKind::Variable(fake_type(11)),
                DeclarationState::Complete,
                DeclarationMode::Fresh,
            )
            .unwrap();
        b = graph.current_environment();

        let (_, consumer) =
            graph.create_detached_region(RegionOwner::Module(ModuleId(2)), Vec::new());
        graph.current = consumer;
        graph.push_overlay(vec![
            LookupEdge::import(b, ModuleId(1)),
            LookupEdge::import(a, ModuleId(0)),
        ]);
        let start = graph.current_environment();

        let ordinary = graph
            .lookup_symbol(start, name, LookupRequest::ORDINARY)
            .unwrap();
        assert_eq!(ordinary.primary[0].symbol, b_value);
        assert_eq!(ordinary.primary[0].imported_from, Some(ModuleId(1)));

        let required_type = graph
            .lookup_symbol(start, name, LookupRequest::REQUIRED_TYPE)
            .unwrap();
        assert_eq!(required_type.primary[0].symbol, a_type);
        assert_eq!(required_type.primary[0].imported_from, Some(ModuleId(0)));
    }

    #[test]
    fn duplicate_names_are_checked_across_frames_of_one_region() {
        let mut graph = ScopeGraph::new();
        graph.enter_region(RegionOwner::Routine(fake_type(0)));
        let name = graph.intern_name("X");
        graph.extend_environment(FrameKind::VarSection);
        let first = graph
            .declare(
                name,
                SymbolKind::Variable(fake_type(1)),
                DeclarationState::Complete,
                DeclarationMode::Fresh,
            )
            .unwrap();
        graph.extend_environment(FrameKind::VarSection);
        assert_eq!(
            graph.declare(
                name,
                SymbolKind::Variable(fake_type(1)),
                DeclarationState::Complete,
                DeclarationMode::Fresh,
            ),
            Err(DeclareError::Duplicate {
                name,
                existing: vec![first],
            })
        );
    }

    #[test]
    fn receiver_layers_use_the_same_ordered_lookup_engine() {
        let mut graph = ScopeGraph::new();
        let name = graph.intern_name("X");
        let other = graph.intern_name("OnlyMember");

        let (_, a_start) =
            graph.create_detached_region(RegionOwner::Type(fake_type(10)), Vec::new());
        graph.select_environment(a_start);
        let a_x = graph
            .declare(
                name,
                SymbolKind::Variable(fake_type(1)),
                DeclarationState::Complete,
                DeclarationMode::Fresh,
            )
            .unwrap();
        let a_members = graph.current_environment();

        let (_, b_start) =
            graph.create_detached_region(RegionOwner::Type(fake_type(11)), Vec::new());
        graph.select_environment(b_start);
        let b_x = graph
            .declare(
                name,
                SymbolKind::Variable(fake_type(1)),
                DeclarationState::Complete,
                DeclarationMode::Fresh,
            )
            .unwrap();
        graph
            .declare(
                other,
                SymbolKind::Variable(fake_type(1)),
                DeclarationState::Complete,
                DeclarationMode::Fresh,
            )
            .unwrap();
        let b_members = graph.current_environment();

        let (_, lexical_start) =
            graph.create_detached_region(RegionOwner::Routine(fake_type(20)), Vec::new());
        graph.select_environment(lexical_start);
        let local_x = graph
            .declare(
                name,
                SymbolKind::Variable(fake_type(1)),
                DeclarationState::Complete,
                DeclarationMode::Fresh,
            )
            .unwrap();
        let locals = graph.current_environment();
        let region = graph.environment_region(locals);

        let with_lookup = graph.create_lookup_environment(
            region,
            vec![
                LookupEdge::with_receiver(b_members, ReceiverId(1)),
                LookupEdge::with_receiver(a_members, ReceiverId(0)),
                LookupEdge::lexical_parent(locals),
            ],
        );
        let with_result = graph
            .lookup_symbol(with_lookup, name, LookupRequest::ORDINARY)
            .unwrap();
        assert_eq!(with_result.primary[0].symbol, b_x);
        assert_eq!(with_result.primary[0].receiver, Some(ReceiverId(1)));
        assert_eq!(with_result.shadowed[0][0].symbol, a_x);
        assert_eq!(with_result.shadowed[1][0].symbol, local_x);

        let method_lookup = graph.create_lookup_environment(
            region,
            vec![
                LookupEdge::lexical_parent(locals),
                LookupEdge::implicit_self(b_members, ReceiverId(2)),
            ],
        );
        let local_result = graph
            .lookup_symbol(method_lookup, name, LookupRequest::ORDINARY)
            .unwrap();
        assert_eq!(local_result.primary[0].symbol, local_x);
        assert_eq!(local_result.primary[0].receiver, None);
        let member_result = graph
            .lookup_symbol(method_lookup, other, LookupRequest::ORDINARY)
            .unwrap();
        assert_eq!(member_result.primary[0].receiver, Some(ReceiverId(2)));
    }
}
