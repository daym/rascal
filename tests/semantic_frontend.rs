use rascal::semantic::bind_sources;

#[test]
fn binds_pointer_forward_and_nested_local_type_in_source_order() {
    let source = "
        program Main;
        procedure Run;
        type
          PNode = ^TNode;
          TNode = record Next: PNode; end;
        var Item: TNode;
        begin
        end;
        begin
        end.
    ";
    let compilation = bind_sources(&[("main.pp", source)]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
    assert_eq!(compilation.files.len(), 1);
    assert_eq!(compilation.files[0].unsupported_declarations, 0);
}

#[test]
fn rejects_pointer_forward_completed_in_a_later_type_section() {
    let source = "
        program Main;
        procedure Run;
        type
          PNode = ^TNode;
        type
          TNode = record Next: PNode; end;
        begin
        end;
        begin
        end.
    ";
    let compilation = bind_sources(&[("main.pp", source)]);
    assert!(
        compilation
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("end of type section")),
        "{:#?}",
        compilation.diagnostics
    );
}

#[test]
fn binds_imported_class_parent_and_alias_across_units() {
    let base = "
        unit Base;
        interface
        type
          TBase = class
            Value: LongInt;
          end;
        implementation
        end.
    ";
    let middle = "
        unit Middle;
        interface
        uses Base;
        type
          TDerived = class(TBase)
          end;
        implementation
        end.
    ";
    let aliases = "
        unit Aliases;
        interface
        uses Middle;
        type
          TAlias = TDerived;
        implementation
        end.
    ";
    let main = "
        program Main;
        uses Aliases;
        var Item: TAlias;
        begin
        end.
    ";
    let compilation = bind_sources(&[
        ("base.pp", base),
        ("middle.pp", middle),
        ("aliases.pp", aliases),
        ("main.pp", main),
    ]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
    assert_eq!(compilation.files.len(), 4);
}

#[test]
fn explicit_class_forward_completes_the_same_declaration() {
    let source = "
        program Main;
        type
          TNode = class;
        type
          TNode = class
            Next: TNode;
          end;
        var Value: TNode;
        begin
        end.
    ";
    let compilation = bind_sources(&[("main.pp", source)]);
    assert!(
        compilation.diagnostics.is_empty(),
        "{:#?}",
        compilation.diagnostics
    );
}
