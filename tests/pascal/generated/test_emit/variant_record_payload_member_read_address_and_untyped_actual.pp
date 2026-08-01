unit u;
interface
type
  plongint = ^longint;
  treference = record
    offset : longint;
    base : longint;
  end;
  tlocation = record
    case tag : longint of
      0 : (reference : treference);
      1 : (reg : longint);
  end;
procedure raw(var x);
function readoffset(var loc : tlocation) : longint;
function addressoffset(var loc : tlocation) : plongint;
procedure passoffset(var loc : tlocation);
implementation
procedure raw(var x);
begin
end;
function readoffset(var loc : tlocation) : longint;
begin
  readoffset := loc.reference.offset;
end;
function addressoffset(var loc : tlocation) : plongint;
begin
  addressoffset := @loc.reference.offset;
end;
procedure passoffset(var loc : tlocation);
begin
  raw(loc.reference.offset);
end;
end.
