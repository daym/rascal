unit u;
interface
type pword = ^word;
procedure run;
implementation
procedure run;
var
  b : array[0..15] of byte;
  i : longint;
  w : word;
begin
  unaligned(pword(@b[i])^) := w;
end;
end.
