unit u;
interface
procedure demo;
implementation
procedure demo;
var b : byte; w : word; d : dword; q : qword; c : cardinal; l : longint;
begin
  b := BsfByte(b) + BsrByte(b);
  c := BsfWord(w) + BsrWord(w);
  c := c + BsfDWord(d) + BsrDWord(d);
  c := c + BsfQWord(q) + BsrQWord(q);
  l := SarLongint(l) + SarLongint(l, 6);
end;
end.
