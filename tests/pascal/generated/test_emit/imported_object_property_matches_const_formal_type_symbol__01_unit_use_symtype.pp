unit use_symtype;
interface
uses symtype;
type
  tbase = object
  private
    _vartype : ttype;
  public
    property vartype : ttype read _vartype write _vartype;
  end;
  tchild = object(tbase)
    procedure writeit(var f : tcompilerppufile);
  end;
implementation
procedure tchild.writeit(var f : tcompilerppufile);
begin
  f.puttype(vartype);
end;
end.
