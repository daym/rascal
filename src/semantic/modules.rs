use super::{EnvironmentId, LookupEdge, ModuleId, NameId, RegionId, RegionOwner, ScopeGraph};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ModulePhase {
    Interface,
    Implementation,
}

#[derive(Clone, Debug)]
pub struct ModuleInfo {
    pub name: NameId,
    /// Contains declarations owned by this interface and no import edges.
    pub interface_exports: EnvironmentId,
    pub interface_uses: Vec<ModuleId>,
    pub implementation_uses: Vec<ModuleId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModuleGraphError {
    InterfaceCycle { cycle: Vec<ModuleId> },
}

#[derive(Clone, Debug, Default)]
pub struct ModuleRegistry {
    modules: Vec<ModuleInfo>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_module(&mut self, name: NameId, interface_exports: EnvironmentId) -> ModuleId {
        let module = ModuleId::from_index(self.modules.len());
        self.modules.push(ModuleInfo {
            name,
            interface_exports,
            interface_uses: Vec::new(),
            implementation_uses: Vec::new(),
        });
        module
    }

    pub fn module(&self, module: ModuleId) -> &ModuleInfo {
        &self.modules[module.index()]
    }

    pub fn set_interface_exports(&mut self, module: ModuleId, exports: EnvironmentId) {
        self.modules[module.index()].interface_exports = exports;
    }

    pub fn set_uses(&mut self, module: ModuleId, phase: ModulePhase, uses: Vec<ModuleId>) {
        match phase {
            ModulePhase::Interface => self.modules[module.index()].interface_uses = uses,
            ModulePhase::Implementation => {
                self.modules[module.index()].implementation_uses = uses;
            }
        }
    }

    /// Returns an order in which every imported interface precedes its user.
    /// Implementation dependencies do not participate, so implementation
    /// cycles remain legal once the interfaces are complete.
    pub fn interface_order(&self) -> Result<Vec<ModuleId>, ModuleGraphError> {
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum Mark {
            Unvisited,
            Visiting,
            Complete,
        }

        fn visit(
            registry: &ModuleRegistry,
            module: ModuleId,
            marks: &mut [Mark],
            stack: &mut Vec<ModuleId>,
            order: &mut Vec<ModuleId>,
        ) -> Result<(), ModuleGraphError> {
            match marks[module.index()] {
                Mark::Complete => return Ok(()),
                Mark::Visiting => {
                    let start = stack
                        .iter()
                        .position(|candidate| *candidate == module)
                        .unwrap_or(0);
                    let mut cycle = stack[start..].to_vec();
                    cycle.push(module);
                    return Err(ModuleGraphError::InterfaceCycle { cycle });
                }
                Mark::Unvisited => {}
            }

            marks[module.index()] = Mark::Visiting;
            stack.push(module);
            for dependency in &registry.module(module).interface_uses {
                visit(registry, *dependency, marks, stack, order)?;
            }
            stack.pop();
            marks[module.index()] = Mark::Complete;
            order.push(module);
            Ok(())
        }

        let mut marks = vec![Mark::Unvisited; self.modules.len()];
        let mut stack = Vec::new();
        let mut order = Vec::with_capacity(self.modules.len());
        for index in 0..self.modules.len() {
            visit(
                self,
                ModuleId::from_index(index),
                &mut marks,
                &mut stack,
                &mut order,
            )?;
        }
        Ok(order)
    }

    pub fn interface_lookup_environment(
        &self,
        scopes: &mut ScopeGraph,
        module: ModuleId,
        local_exports: EnvironmentId,
        system_exports: Option<(ModuleId, EnvironmentId)>,
    ) -> EnvironmentId {
        let region = scopes.environment_region(local_exports);
        let mut layers = vec![LookupEdge::lexical_parent(local_exports)];
        layers.extend(self.import_edges(&self.module(module).interface_uses));
        if let Some((system, exports)) = system_exports {
            layers.push(LookupEdge::system(exports, system));
        }
        scopes.create_lookup_environment(region, layers)
    }

    pub fn implementation_lookup_environment(
        &self,
        scopes: &mut ScopeGraph,
        module: ModuleId,
        implementation_locals: EnvironmentId,
        system_exports: Option<(ModuleId, EnvironmentId)>,
    ) -> EnvironmentId {
        let info = self.module(module);
        let region = scopes.environment_region(implementation_locals);
        let mut layers = vec![
            LookupEdge::lexical_parent(implementation_locals),
            LookupEdge::lexical_parent(info.interface_exports),
        ];
        layers.extend(self.import_edges(&info.implementation_uses));
        layers.extend(self.import_edges(&info.interface_uses));
        if let Some((system, exports)) = system_exports {
            layers.push(LookupEdge::system(exports, system));
        }
        scopes.create_lookup_environment(region, layers)
    }

    fn import_edges<'a>(
        &'a self,
        source_order: &'a [ModuleId],
    ) -> impl Iterator<Item = LookupEdge> + 'a {
        source_order.iter().rev().map(move |unit| {
            let exports = self.module(*unit).interface_exports;
            LookupEdge::import(exports, *unit)
        })
    }
}

pub fn create_module_export_environment(
    scopes: &mut ScopeGraph,
    module: ModuleId,
) -> (RegionId, EnvironmentId) {
    scopes.create_detached_region(RegionOwner::Module(module), Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantic::{
        DeclarationMode, DeclarationState, LookupRequest, SymbolId, SymbolKind, TypeRef,
    };

    fn declare_value(
        scopes: &mut ScopeGraph,
        environment: EnvironmentId,
        name: NameId,
        ty: TypeRef,
    ) -> (EnvironmentId, SymbolId) {
        scopes.select_environment(environment);
        let symbol = scopes
            .declare(
                name,
                SymbolKind::Variable(ty),
                DeclarationState::Complete,
                DeclarationMode::Fresh,
            )
            .unwrap();
        (scopes.current_environment(), symbol)
    }

    #[test]
    fn rightmost_use_wins_without_transitive_reexport() {
        let mut scopes = ScopeGraph::new();
        let mut modules = ModuleRegistry::new();
        let x = scopes.intern_name("X");

        let (_, a_start) =
            scopes.create_detached_region(RegionOwner::Module(ModuleId(0)), Vec::new());
        let a_name = scopes.intern_name("A");
        let a = modules.add_module(a_name, a_start);
        let (a_exports, a_x) = declare_value(&mut scopes, a_start, x, TypeRef(1));
        modules.set_interface_exports(a, a_exports);

        let (_, b_start) =
            scopes.create_detached_region(RegionOwner::Module(ModuleId(1)), Vec::new());
        let b_name = scopes.intern_name("B");
        let b = modules.add_module(b_name, b_start);
        let (b_exports, b_x) = declare_value(&mut scopes, b_start, x, TypeRef(2));
        modules.set_interface_exports(b, b_exports);

        let (_, c_exports) =
            scopes.create_detached_region(RegionOwner::Module(ModuleId(2)), Vec::new());
        let c_name = scopes.intern_name("C");
        let c = modules.add_module(c_name, c_exports);
        modules.set_uses(c, ModulePhase::Interface, vec![a, b]);
        let lookup = modules.interface_lookup_environment(&mut scopes, c, c_exports, None);

        let result = scopes
            .lookup_symbol(lookup, x, LookupRequest::ORDINARY)
            .unwrap();
        assert_eq!(result.primary[0].symbol, b_x);
        assert_eq!(result.shadowed[0][0].symbol, a_x);

        let (_, d_exports) =
            scopes.create_detached_region(RegionOwner::Module(ModuleId(3)), Vec::new());
        let d_name = scopes.intern_name("D");
        let d = modules.add_module(d_name, d_exports);
        modules.set_uses(d, ModulePhase::Interface, vec![c]);
        let d_lookup = modules.interface_lookup_environment(&mut scopes, d, d_exports, None);
        assert!(
            scopes
                .lookup_symbol(d_lookup, x, LookupRequest::ORDINARY)
                .is_none()
        );
    }

    #[test]
    fn local_declaration_beats_imports() {
        let mut scopes = ScopeGraph::new();
        let mut modules = ModuleRegistry::new();
        let x = scopes.intern_name("X");

        let (_, a_start) =
            scopes.create_detached_region(RegionOwner::Module(ModuleId(0)), Vec::new());
        let a_name = scopes.intern_name("A");
        let a = modules.add_module(a_name, a_start);
        let (a_exports, _) = declare_value(&mut scopes, a_start, x, TypeRef(1));
        modules.set_interface_exports(a, a_exports);

        let (_, b_start) =
            scopes.create_detached_region(RegionOwner::Module(ModuleId(1)), Vec::new());
        let b_name = scopes.intern_name("B");
        let b = modules.add_module(b_name, b_start);
        let (b_exports, local_x) = declare_value(&mut scopes, b_start, x, TypeRef(2));
        modules.set_interface_exports(b, b_exports);
        modules.set_uses(b, ModulePhase::Interface, vec![a]);
        let lookup = modules.interface_lookup_environment(&mut scopes, b, b_exports, None);

        let result = scopes
            .lookup_symbol(lookup, x, LookupRequest::ORDINARY)
            .unwrap();
        assert_eq!(result.primary[0].symbol, local_x);
    }

    #[test]
    fn interface_cycles_fail_but_implementation_cycles_are_accepted() {
        let mut scopes = ScopeGraph::new();
        let mut modules = ModuleRegistry::new();
        let (_, a_exports) =
            scopes.create_detached_region(RegionOwner::Module(ModuleId(0)), Vec::new());
        let a_name = scopes.intern_name("A");
        let a = modules.add_module(a_name, a_exports);
        let (_, b_exports) =
            scopes.create_detached_region(RegionOwner::Module(ModuleId(1)), Vec::new());
        let b_name = scopes.intern_name("B");
        let b = modules.add_module(b_name, b_exports);

        modules.set_uses(a, ModulePhase::Implementation, vec![b]);
        modules.set_uses(b, ModulePhase::Implementation, vec![a]);
        assert_eq!(modules.interface_order().unwrap(), vec![a, b]);

        modules.set_uses(a, ModulePhase::Interface, vec![b]);
        modules.set_uses(b, ModulePhase::Interface, vec![a]);
        assert!(matches!(
            modules.interface_order(),
            Err(ModuleGraphError::InterfaceCycle { .. })
        ));
    }
}
