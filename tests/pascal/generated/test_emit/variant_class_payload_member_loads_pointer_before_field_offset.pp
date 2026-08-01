unit u;
interface
type
  psymtable = ^tsymtable;
  tsymtable = record
    marker : longint;
  end;
  tsym = class
  public
    owner : psymtable;
  end;
  pitem = ^titem;
  titem = record
    case byte of
      0 : (sym : tsym);
      1 : (value : longint);
  end;
function read_owner(plist : pitem) : psymtable;
procedure write_owner(plist : pitem; st : psymtable);
implementation
function read_owner(plist : pitem) : psymtable;
begin
  read_owner := plist^.sym.owner;
end;
procedure write_owner(plist : pitem; st : psymtable);
begin
  plist^.sym.owner := st;
end;
end.
