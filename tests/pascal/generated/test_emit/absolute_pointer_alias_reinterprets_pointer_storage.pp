unit u;
interface
type
  pint = ^longint;
procedure demo(raw : pointer);
implementation
procedure demo(raw : pointer);
var
  value : pint absolute raw;
begin
  writeln(value^);
end;
end.
