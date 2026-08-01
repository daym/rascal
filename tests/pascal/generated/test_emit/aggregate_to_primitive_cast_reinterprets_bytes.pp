unit u;
interface
type
  tdoublearray = array[0..7] of byte;
const
  bits : tdoublearray = (0,0,0,0,0,0,240,127);
procedure fetch;
implementation
procedure fetch;
var
  d : double;
begin
  d := double(bits);
end;
end.
