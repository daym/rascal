unit u;
interface
type pword = ^word;
procedure run;
implementation
procedure run;
var
  b : array[0..15] of byte;
  i : longint;
  n : word;
begin
  inc(unaligned(pword(@b[i])^), n);
  dec(unaligned(pword(@b[i])^));
end;
end.
