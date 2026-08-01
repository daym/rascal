unit u;
interface
type
  P1 = ^Integer;
  Q1 = ^Integer;
  A = ^P1;
  B = ^Q1;
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
