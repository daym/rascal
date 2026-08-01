unit u;
interface
type
  tarr = array[0..7] of longint;
procedure poke(var b; i : longint);
implementation
procedure poke(var b; i : longint);
begin
  tarr(b)[i] := tarr(b)[i];
end;
end.
