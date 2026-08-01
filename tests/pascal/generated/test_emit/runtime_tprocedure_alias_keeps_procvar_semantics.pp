unit u;
interface
var
  hook : TProcedure;
procedure foo;
procedure demo;
implementation
procedure foo;
begin
end;
procedure demo;
begin
  hook := foo;
  if assigned(hook) then
    hook;
end;
end.
