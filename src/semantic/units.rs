use super::{EnvironmentId, LookupEdge, NameId, RegionId, RegionOwner, ScopeGraph, UnitId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnitPhase {
    Interface,
    Implementation,
}

#[derive(Clone, Debug)]
pub struct UnitInfo {
    pub name: NameId,
    /// Contains declarations owned by this interface and no import edges.
    pub interface_exports: EnvironmentId,
    pub interface_uses: Vec<UnitId>,
    pub implementation_uses: Vec<UnitId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UnitGraphError {
    InterfaceCycle { cycle: Vec<UnitId> },
}

#[derive(Clone, Debug, Default)]
pub struct UnitRegistry {
    units: Vec<UnitInfo>,
}

impl UnitRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_unit(&mut self, name: NameId, interface_exports: EnvironmentId) -> UnitId {
        let unit = UnitId::from_index(self.units.len());
        self.units.push(UnitInfo {
            name,
            interface_exports,
            interface_uses: Vec::new(),
            implementation_uses: Vec::new(),
        });
        unit
    }

    pub fn unit(&self, unit: UnitId) -> &UnitInfo {
        &self.units[unit.index()]
    }

    pub fn set_interface_exports(&mut self, unit: UnitId, exports: EnvironmentId) {
        self.units[unit.index()].interface_exports = exports;
    }

    pub fn set_uses(&mut self, unit: UnitId, phase: UnitPhase, uses: Vec<UnitId>) {
        match phase {
            UnitPhase::Interface => self.units[unit.index()].interface_uses = uses,
            UnitPhase::Implementation => self.units[unit.index()].implementation_uses = uses,
        }
    }

    /// Returns an order in which every imported interface precedes its user.
    /// Implementation dependencies do not participate, so implementation
    /// cycles remain legal once the interfaces are complete.
    pub fn interface_order(&self) -> Result<Vec<UnitId>, UnitGraphError> {
        #[derive(Clone, Copy, PartialEq, Eq)]
        enum Mark {
            Unvisited,
            Visiting,
            Complete,
        }

        fn visit(
            registry: &UnitRegistry,
            unit: UnitId,
            marks: &mut [Mark],
            stack: &mut Vec<UnitId>,
            order: &mut Vec<UnitId>,
        ) -> Result<(), UnitGraphError> {
            match marks[unit.index()] {
                Mark::Complete => return Ok(()),
                Mark::Visiting => {
                    let start = stack
                        .iter()
                        .position(|candidate| *candidate == unit)
                        .unwrap_or(0);
                    let mut cycle = stack[start..].to_vec();
                    cycle.push(unit);
                    return Err(UnitGraphError::InterfaceCycle { cycle });
                }
                Mark::Unvisited => {}
            }

            marks[unit.index()] = Mark::Visiting;
            stack.push(unit);
            for dependency in &registry.unit(unit).interface_uses {
                visit(registry, *dependency, marks, stack, order)?;
            }
            stack.pop();
            marks[unit.index()] = Mark::Complete;
            order.push(unit);
            Ok(())
        }

        let mut marks = vec![Mark::Unvisited; self.units.len()];
        let mut stack = Vec::new();
        let mut order = Vec::with_capacity(self.units.len());
        for index in 0..self.units.len() {
            visit(
                self,
                UnitId::from_index(index),
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
        unit: UnitId,
        local_exports: EnvironmentId,
        system_exports: Option<(UnitId, EnvironmentId)>,
    ) -> EnvironmentId {
        let region = scopes.environment_region(local_exports);
        let mut layers = vec![LookupEdge::lexical_parent(local_exports)];
        layers.extend(self.import_edges(&self.unit(unit).interface_uses));
        if let Some((system, exports)) = system_exports {
            layers.push(LookupEdge::system(exports, system));
        }
        scopes.create_lookup_environment(region, layers)
    }

    pub fn implementation_lookup_environment(
        &self,
        scopes: &mut ScopeGraph,
        unit: UnitId,
        implementation_locals: EnvironmentId,
        system_exports: Option<(UnitId, EnvironmentId)>,
    ) -> EnvironmentId {
        let info = self.unit(unit);
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
        source_order: &'a [UnitId],
    ) -> impl Iterator<Item = LookupEdge> + 'a {
        source_order.iter().rev().map(move |unit| {
            let exports = self.unit(*unit).interface_exports;
            LookupEdge::import(exports, *unit)
        })
    }
}

pub fn create_unit_export_environment(
    scopes: &mut ScopeGraph,
    unit: UnitId,
) -> (RegionId, EnvironmentId) {
    scopes.create_detached_region(RegionOwner::Unit(unit), Vec::new())
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
        let mut units = UnitRegistry::new();
        let x = scopes.intern_name("X");

        let (_, a_start) = scopes.create_detached_region(RegionOwner::Unit(UnitId(0)), Vec::new());
        let a_name = scopes.intern_name("A");
        let a = units.add_unit(a_name, a_start);
        let (a_exports, a_x) = declare_value(&mut scopes, a_start, x, TypeRef(1));
        units.set_interface_exports(a, a_exports);

        let (_, b_start) = scopes.create_detached_region(RegionOwner::Unit(UnitId(1)), Vec::new());
        let b_name = scopes.intern_name("B");
        let b = units.add_unit(b_name, b_start);
        let (b_exports, b_x) = declare_value(&mut scopes, b_start, x, TypeRef(2));
        units.set_interface_exports(b, b_exports);

        let (_, c_exports) =
            scopes.create_detached_region(RegionOwner::Unit(UnitId(2)), Vec::new());
        let c_name = scopes.intern_name("C");
        let c = units.add_unit(c_name, c_exports);
        units.set_uses(c, UnitPhase::Interface, vec![a, b]);
        let lookup = units.interface_lookup_environment(&mut scopes, c, c_exports, None);

        let result = scopes
            .lookup_symbol(lookup, x, LookupRequest::ORDINARY)
            .unwrap();
        assert_eq!(result.primary[0].symbol, b_x);
        assert_eq!(result.shadowed[0][0].symbol, a_x);

        let (_, d_exports) =
            scopes.create_detached_region(RegionOwner::Unit(UnitId(3)), Vec::new());
        let d_name = scopes.intern_name("D");
        let d = units.add_unit(d_name, d_exports);
        units.set_uses(d, UnitPhase::Interface, vec![c]);
        let d_lookup = units.interface_lookup_environment(&mut scopes, d, d_exports, None);
        assert!(
            scopes
                .lookup_symbol(d_lookup, x, LookupRequest::ORDINARY)
                .is_none()
        );
    }

    #[test]
    fn local_declaration_beats_imports() {
        let mut scopes = ScopeGraph::new();
        let mut units = UnitRegistry::new();
        let x = scopes.intern_name("X");

        let (_, a_start) = scopes.create_detached_region(RegionOwner::Unit(UnitId(0)), Vec::new());
        let a_name = scopes.intern_name("A");
        let a = units.add_unit(a_name, a_start);
        let (a_exports, _) = declare_value(&mut scopes, a_start, x, TypeRef(1));
        units.set_interface_exports(a, a_exports);

        let (_, b_start) = scopes.create_detached_region(RegionOwner::Unit(UnitId(1)), Vec::new());
        let b_name = scopes.intern_name("B");
        let b = units.add_unit(b_name, b_start);
        let (b_exports, local_x) = declare_value(&mut scopes, b_start, x, TypeRef(2));
        units.set_interface_exports(b, b_exports);
        units.set_uses(b, UnitPhase::Interface, vec![a]);
        let lookup = units.interface_lookup_environment(&mut scopes, b, b_exports, None);

        let result = scopes
            .lookup_symbol(lookup, x, LookupRequest::ORDINARY)
            .unwrap();
        assert_eq!(result.primary[0].symbol, local_x);
    }

    #[test]
    fn interface_cycles_fail_but_implementation_cycles_are_accepted() {
        let mut scopes = ScopeGraph::new();
        let mut units = UnitRegistry::new();
        let (_, a_exports) =
            scopes.create_detached_region(RegionOwner::Unit(UnitId(0)), Vec::new());
        let a_name = scopes.intern_name("A");
        let a = units.add_unit(a_name, a_exports);
        let (_, b_exports) =
            scopes.create_detached_region(RegionOwner::Unit(UnitId(1)), Vec::new());
        let b_name = scopes.intern_name("B");
        let b = units.add_unit(b_name, b_exports);

        units.set_uses(a, UnitPhase::Implementation, vec![b]);
        units.set_uses(b, UnitPhase::Implementation, vec![a]);
        assert_eq!(units.interface_order().unwrap(), vec![a, b]);

        units.set_uses(a, UnitPhase::Interface, vec![b]);
        units.set_uses(b, UnitPhase::Interface, vec![a]);
        assert!(matches!(
            units.interface_order(),
            Err(UnitGraphError::InterfaceCycle { .. })
        ));
    }
}
