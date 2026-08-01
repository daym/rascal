unit u;
interface
type
  A = ^Integer;
  B = ^Integer;
procedure take(var p : B);
var
  x : A;
implementation
procedure take(var p : B);
begin
end;
begin
  take(x);
end.
