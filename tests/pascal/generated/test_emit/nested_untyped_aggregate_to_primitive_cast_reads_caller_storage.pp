unit u;
interface
procedure fetch(var b);
implementation
procedure fetch(var b);
type
  tdummyarray = packed array[0..7] of byte;
var
  d : double;
begin
  d := double(tdummyarray(b));
end;
end.
