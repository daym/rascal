unit u;
interface
procedure demo;
implementation
procedure demo;
type
  pint = ^longint;
  parr = array[0..1] of pint;
const
  raw : array[0..1] of pointer = (nil, nil);
var
  view : parr absolute raw;
begin
  writeln(view[0]);
end;
end.
