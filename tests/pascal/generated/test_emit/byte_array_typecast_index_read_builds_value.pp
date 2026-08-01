unit u;
interface
type
  t80bitarray = array[0..9] of byte;
procedure dump(e : extended; i : longint);
implementation
procedure dump(e : extended; i : longint);
begin
  writeln(t80bitarray(e)[i]);
end;
end.
