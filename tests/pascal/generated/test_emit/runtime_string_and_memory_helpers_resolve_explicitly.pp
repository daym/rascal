unit u;
interface
procedure demo;
implementation
procedure demo;
var s : string; p : pchar; c : char; b : boolean; x : longint;
begin
  fillbyte(s, sizeof(s), 0);
  initialize(x);
  b := directoryexists(s);
  s := inttostr(x);
  setstring(s, p, x);
  s := trim(s);
  p := reallocmem(p, x);
  p := strrscan(p, c);
end;
end.
