unit u;
interface
type
  tarr = array[0..7] of longint;
procedure poke(var b; i : longint; v : longint);
implementation
procedure poke(var b; i : longint; v : longint);
begin
  tarr(b)[i] := v;
end;
end.
