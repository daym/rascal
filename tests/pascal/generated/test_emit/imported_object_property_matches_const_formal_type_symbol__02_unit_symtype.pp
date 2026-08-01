unit symtype;
interface
type
  ttype = object
    p : pointer;
  end;
  tcompilerppufile = object
    procedure puttype(const t : ttype);
  end;
implementation
procedure tcompilerppufile.puttype(const t : ttype);
begin
end;
end.
