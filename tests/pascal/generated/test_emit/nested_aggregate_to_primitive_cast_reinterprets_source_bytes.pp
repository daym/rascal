unit u;
interface
procedure fetch;
implementation
procedure fetch;
type
  tdummyarray = packed array[0..7] of byte;
const
  dummy1 : int64 = $4330000080000000;
var
  a : tdummyarray;
  d : double;
begin
  a := tdummyarray(dummy1);
  d := double(tdummyarray(dummy1));
end;
end.
