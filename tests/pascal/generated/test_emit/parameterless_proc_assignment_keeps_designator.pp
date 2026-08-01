unit u;
interface
type
  tproc = procedure;
var
  p : tproc;
procedure foo;
procedure demo;
implementation
procedure foo;
begin
end;
procedure demo;
begin
  p := foo;
end;
end.
