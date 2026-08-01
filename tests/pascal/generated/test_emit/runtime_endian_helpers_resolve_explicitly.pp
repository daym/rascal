unit u;
interface
procedure demo;
implementation
procedure demo;
var l : longint; w : word;
begin
  l := NtoBE(l) + BEtoN(l) + NtoLE(l) + LEtoN(l);
  w := NtoBE(w);
end;
end.
