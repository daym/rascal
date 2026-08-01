unit u;
interface
type
  tnode = class
  end;
  ttype = object
    p : pointer;
  end;
  tabstractvarsym = class
  private
    _vartype : ttype;
    procedure setvartype(const newtype : ttype);
  public
    property vartype : ttype read _vartype write setvartype;
  end;
  tparavarsym = class(tabstractvarsym)
  end;
  tunarynode = class
  public
    left : tnode;
  end;
  tcallparanode = class(tunarynode)
  public
    parasym : tparavarsym;
    procedure insert_typeconv;
  end;
procedure takevar(var p : tnode);
procedure takeconst(const t : ttype);
procedure inserttypeconv(var p : tnode; const t : ttype);
implementation
procedure tabstractvarsym.setvartype(const newtype : ttype);
begin
  _vartype := newtype;
end;
procedure takevar(var p : tnode);
begin
end;
procedure takeconst(const t : ttype);
begin
end;
procedure inserttypeconv(var p : tnode; const t : ttype);
begin
end;
procedure tcallparanode.insert_typeconv;
begin
  takevar(left);
  takeconst(parasym.vartype);
  inserttypeconv(left, parasym.vartype);
end;
end.
