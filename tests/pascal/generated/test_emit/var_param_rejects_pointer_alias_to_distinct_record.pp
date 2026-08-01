unit u;
interface
type
  X = Integer;
  R1 = record p : ^X; end;
  R2 = record p : ^X; end;
  A = ^R1;
  B = ^R2;
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
