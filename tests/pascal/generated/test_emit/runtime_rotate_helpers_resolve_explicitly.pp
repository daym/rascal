unit u;
interface
procedure demo;
implementation
procedure demo;
var b : byte; w : word; d : dword; q : qword;
begin
  b := RorByte(b) + RolByte(b, 3);
  w := RorWord(w) + RolWord(w, 3);
  d := RorDWord(d) + RolDWord(d, 3);
  q := RorQWord(q) + RolQWord(q, 3);
end;
end.
