unit u;
interface
type
  treg = 0..7;
  tregs = set of treg;
procedure p;
implementation
procedure p;
var regs : tregs; j : treg; total : integer;
begin
  total := 0;
  for j in regs do
    total := total + j;
end;
end.
