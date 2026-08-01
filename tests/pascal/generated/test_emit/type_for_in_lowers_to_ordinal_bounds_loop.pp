unit u;
interface
type
  tkind = (ka, kb, kc);
procedure p;
implementation
procedure p;
var k : tkind; total : integer;
begin
  total := 0;
  for k in tkind do
    total := total + 1;
end;
end.
