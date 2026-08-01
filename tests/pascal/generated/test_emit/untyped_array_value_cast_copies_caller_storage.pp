unit u;
interface
type
  tarr = array[0..7] of byte;
procedure copy(var b);
implementation
procedure copy(var b);
var
  a : tarr;
begin
  a := tarr(b);
end;
end.
