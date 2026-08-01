unit u;
interface
type
  A = 0..2;
  B = 0..2;
  TA = array[A] of string[7];
  TB = array[B] of string[8];
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
