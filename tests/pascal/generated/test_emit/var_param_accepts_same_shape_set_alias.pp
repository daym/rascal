unit u;
interface
type
  A = 0..2;
  B = 0..2;
  SA = set of A;
  SB = set of B;
procedure take(var x : SB);
var
  v : SA;
implementation
procedure take(var x : SB);
begin
end;
begin
  take(v);
end.
