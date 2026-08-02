use rascal::semantic::{
    ApplicationReceiver, BoundApplicationTarget, BoundExpressionKind, BoundStatementKind,
    CallableFlavor, LookupRequest, MethodDispatch, SymbolId, SymbolKind, TypeRef, bind_sources,
};

fn named_type(compilation: &rascal::semantic::SemanticCompilation, spelling: &str) -> TypeRef {
    let name = compilation.binder.scopes.names().lookup(spelling).unwrap();
    let lookup = compilation
        .binder
        .scopes
        .lookup_symbol(
            compilation.files[0].environment,
            name,
            LookupRequest::REQUIRED_TYPE,
        )
        .unwrap();
    let SymbolKind::Type(ty) = compilation
        .binder
        .scopes
        .symbol(lookup.primary[0].symbol)
        .kind
    else {
        panic!("expected type `{spelling}`")
    };
    ty
}

fn member(
    compilation: &rascal::semantic::SemanticCompilation,
    owner: TypeRef,
    spelling: &str,
) -> SymbolId {
    let name = compilation.binder.scopes.names().lookup(spelling).unwrap();
    let environment = compilation.binder.types.member_environment(owner).unwrap();
    compilation
        .binder
        .scopes
        .lookup_symbol(environment, name, LookupRequest::ORDINARY)
        .unwrap()
        .primary[0]
        .symbol
}

#[test]
fn fpc_style_class_methods_overrides_overloads_and_inherited_bind_together() {
    // The class-function/override shape is adapted from compiler/optdead.pas.
    let source = "
        program ClassSlice;
        type
          TBase = class
            function Select(N: LongInt): LongInt; overload; virtual;
            class function Kind: LongInt; virtual;
            class procedure Configure(N: LongInt); virtual;
            procedure Touch(N: LongInt); virtual;
          end;
          TChild = class(TBase)
            function Select(B: Boolean): LongInt; overload;
            class function Kind: LongInt; override;
            class procedure Configure(N: LongInt); override;
            procedure Touch(N: LongInt); override;
          end;

        function TBase.Select(N: LongInt): LongInt;
        begin
          Result := N;
        end;

        class function TBase.Kind: LongInt;
        begin
          Result := 1;
        end;

        class procedure TBase.Configure(N: LongInt);
        begin
        end;

        procedure TBase.Touch(N: LongInt);
        begin
        end;

        function TChild.Select(B: Boolean): LongInt;
        begin
          Result := 2;
        end;

        class function TChild.Kind: LongInt;
        begin
          Result := inherited Kind + 1;
        end;

        class procedure TChild.Configure(N: LongInt);
        begin
          inherited Configure(N);
        end;

        procedure TChild.Touch(N: LongInt);
        begin
          inherited;
        end;

        var
          Item: TChild;
          Number: LongInt;
        begin
          Number := Item.Select(1);
          Number := Item.Select(True);
          Number := TChild.Kind;
          Item.Touch(7);
          TChild.Configure(3);
        end.
    ";
    let compilation = bind_sources(&[("class_slice.pp", source)]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );

    let base = named_type(&compilation, "tbase");
    let child = named_type(&compilation, "tchild");
    let base_kind = member(&compilation, base, "kind");
    let child_kind = member(&compilation, child, "kind");
    let SymbolKind::Routine(base_kind_type) = compilation.binder.scopes.symbol(base_kind).kind
    else {
        panic!("base Kind must be callable")
    };
    let SymbolKind::Routine(child_kind_type) = compilation.binder.scopes.symbol(child_kind).kind
    else {
        panic!("child Kind must be callable")
    };
    let base_callable = compilation.binder.types.callable(base_kind_type).unwrap();
    let child_callable = compilation.binder.types.callable(child_kind_type).unwrap();
    assert_eq!(base_callable.flavor, CallableFlavor::ClassMethod);
    assert_eq!(child_callable.flavor, CallableFlavor::ClassMethod);
    let MethodDispatch::Virtual {
        slot: base_slot, ..
    } = base_callable.method.unwrap().dispatch
    else {
        panic!("base Kind must introduce a virtual slot")
    };
    let MethodDispatch::Virtual {
        slot: child_slot,
        overridden,
    } = child_callable.method.unwrap().dispatch
    else {
        panic!("child Kind must override a virtual slot")
    };
    assert_eq!(child_slot, base_slot);
    assert_eq!(overridden, Some(base_kind));
    assert_eq!(child_callable.method.unwrap().ancestor(), Some(base_kind));

    let program = compilation
        .bodies
        .iter()
        .find(|body| body.owner.is_none())
        .unwrap();
    let base_select = member(&compilation, base, "select");
    let BoundStatementKind::Assignment(inherited_overload_call) = &program.statements[0].kind
    else {
        panic!("expected inherited-overload assignment")
    };
    let BoundExpressionKind::Application {
        target: BoundApplicationTarget::Routine { resolution },
        ..
    } = &inherited_overload_call.source.kind
    else {
        panic!("expected inherited-overload application")
    };
    assert_eq!(resolution.selected_symbol(), Some(base_select));

    let BoundStatementKind::Assignment(class_call) = &program.statements[2].kind else {
        panic!("expected class-function assignment")
    };
    let BoundExpressionKind::Application {
        target: BoundApplicationTarget::Routine { resolution },
        ..
    } = &class_call.source.kind
    else {
        panic!("expected class-function application")
    };
    assert!(matches!(
        resolution.selected_attempt().unwrap().candidate.receiver(),
        ApplicationReceiver::ClassIdentifier(ty) if ty == child
    ));

    let child_touch = member(&compilation, child, "touch");
    let SymbolKind::Routine(child_touch_type) = compilation.binder.scopes.symbol(child_touch).kind
    else {
        panic!("child Touch must be callable")
    };
    let touch_body = compilation
        .bodies
        .iter()
        .find(|body| body.owner == Some(child_touch_type))
        .unwrap();
    let BoundStatementKind::Expression(inherited) = &touch_body.statements[0].kind else {
        panic!("expected inherited statement")
    };
    let BoundExpressionKind::Application {
        target: BoundApplicationTarget::Routine { resolution },
        operands,
        ..
    } = &inherited.kind
    else {
        panic!("expected inherited application")
    };
    assert_eq!(operands.len(), 1);
    assert_eq!(
        resolution.selected_attempt().unwrap().candidate.receiver(),
        ApplicationReceiver::Inherited
    );
}

#[test]
fn old_object_virtual_means_introduce_or_replace_and_override_is_rejected() {
    let valid = "
        program ObjectSlice;
        type
          TBase = object
            procedure Touch(N: LongInt); virtual;
          end;
          TChild = object(TBase)
            procedure Touch(N: LongInt); virtual;
          end;

        procedure TBase.Touch(N: LongInt);
        begin
        end;

        procedure TChild.Touch(N: LongInt);
        begin
          inherited;
        end;

        var Item: TChild;
        begin
          Item.Touch(1);
        end.
    ";
    let compilation = bind_sources(&[("object_slice.pp", valid)]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
    let base = named_type(&compilation, "tbase");
    let child = named_type(&compilation, "tchild");
    let base_touch = member(&compilation, base, "touch");
    let child_touch = member(&compilation, child, "touch");
    let SymbolKind::Routine(base_touch_type) = compilation.binder.scopes.symbol(base_touch).kind
    else {
        panic!("base Touch must be callable")
    };
    let SymbolKind::Routine(child_touch_type) = compilation.binder.scopes.symbol(child_touch).kind
    else {
        panic!("child Touch must be callable")
    };
    let base_method = compilation
        .binder
        .types
        .callable(base_touch_type)
        .unwrap()
        .method
        .unwrap();
    let child_method = compilation
        .binder
        .types
        .callable(child_touch_type)
        .unwrap()
        .method
        .unwrap();
    assert_eq!(child_method.virtual_slot(), base_method.virtual_slot());
    assert_eq!(child_method.overridden(), Some(base_touch));
    assert_eq!(child_method.ancestor(), Some(base_touch));

    let invalid = "
        program InvalidObjectOverride;
        type
          TBase = object
            procedure Touch; virtual;
          end;
          TChild = object(TBase)
            procedure Touch; override;
          end;
        begin
        end.
    ";
    let compilation = bind_sources(&[("invalid_object_override.pp", invalid)]);
    assert!(
        compilation.diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("old-style object methods support only the `virtual`")),
        "{:#?}",
        compilation.diagnostics
    );

    let missing_virtual = "
        program InvalidObjectReplacement;
        type
          TBase = object
            procedure Touch; virtual;
          end;
          TChild = object(TBase)
            procedure Touch;
          end;
        begin
        end.
    ";
    let compilation = bind_sources(&[("invalid_object_replacement.pp", missing_virtual)]);
    assert!(
        compilation.diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("object method must be declared `virtual`")),
        "{:#?}",
        compilation.diagnostics
    );
}

#[test]
fn class_override_requires_an_exact_inherited_virtual_signature() {
    let source = "
        program InvalidOverrides;
        type
          TBase = class
            procedure Touch(N: LongInt); virtual;
          end;
          TMissing = class(TBase)
            procedure Other; override;
          end;
          THiding = class(TBase)
            procedure Touch(N: LongInt); virtual;
          end;
        begin
        end.
    ";
    let compilation = bind_sources(&[("invalid_overrides.pp", source)]);
    assert!(
        compilation.diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("override has no matching inherited virtual method")),
        "{:#?}",
        compilation.diagnostics
    );
    assert!(
        compilation.diagnostics.iter().any(|diagnostic| diagnostic
            .message
            .contains("matching inherited virtual method must use `override`")),
        "{:#?}",
        compilation.diagnostics
    );
}

#[test]
fn virtual_slots_survive_an_empty_intermediate_class() {
    let source = "
        program StableSlots;
        type
          TBase = class
            procedure First; virtual;
          end;
          TMiddle = class(TBase)
          end;
          TLeaf = class(TMiddle)
            procedure Second; virtual;
          end;
        begin
        end.
    ";
    let compilation = bind_sources(&[("stable_slots.pp", source)]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
    let base = named_type(&compilation, "tbase");
    let leaf = named_type(&compilation, "tleaf");
    let base_method = member(&compilation, base, "first");
    let leaf_method = member(&compilation, leaf, "second");
    let SymbolKind::Routine(base_method_type) = compilation.binder.scopes.symbol(base_method).kind
    else {
        panic!("base method must be callable")
    };
    let SymbolKind::Routine(leaf_method_type) = compilation.binder.scopes.symbol(leaf_method).kind
    else {
        panic!("leaf method must be callable")
    };
    assert_eq!(
        compilation
            .binder
            .types
            .callable(base_method_type)
            .unwrap()
            .method
            .unwrap()
            .virtual_slot(),
        Some(0)
    );
    assert_eq!(
        compilation
            .binder
            .types
            .callable(leaf_method_type)
            .unwrap()
            .method
            .unwrap()
            .virtual_slot(),
        Some(1)
    );
}
