unit u;
interface
type
  treference = record
    offset : longint;
    base : longint;
  end;
  tlocation = record
    case tag : longint of
      0 : (reference : treference);
      1 : (reg : longint);
  end;
procedure touch(var r : treference);
procedure run(var loc : tlocation; delta : longint);
implementation
procedure touch(var r : treference);
begin
end;
procedure run(var loc : tlocation; delta : longint);
begin
  loc.reference.offset := 4;
  inc(loc.reference.offset, delta);
  touch(loc.reference);
end;
end.
