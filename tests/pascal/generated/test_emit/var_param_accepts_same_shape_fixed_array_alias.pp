unit u;
interface
type
  A = 0..2;
  B = 0..2;
  TA = array[A] of longint;
  TB = array[B] of longint;
procedure take(var x : TB);
var
  v : TA;
implementation
procedure take(var x : TB);
begin
end;
begin
  take(v);
end.
