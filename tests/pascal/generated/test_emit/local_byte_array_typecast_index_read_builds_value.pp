unit u;
interface
procedure dump(e : extended; i : longint);
implementation
procedure dump(e : extended; i : longint);
type
  t80bitarray = array[0..9] of byte;
begin
  writeln(t80bitarray(e)[i]);
end;
end.
