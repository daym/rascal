unit u;
interface
type
  tasm = (as_none, as_gas, as_ggas, as_darwin);
procedure p;
implementation
procedure p;
var asmkind : tasm; total : longint;
begin
  total := 0;
  for asmkind in [as_gas, as_ggas, as_darwin] do
    total := total + ord(asmkind);
end;
end.
