unit u;
interface
type
  bestreal = extended;
  pbestreal = ^bestreal;
procedure demo;
implementation
procedure demo;
var n : longint; p : pbestreal;
begin
  n := sizeof(pbestreal^);
  n := sizeof(p^);
end;
end.
