unit u;
interface
type
  pword = ^word;
  plongint = ^longint;
procedure take(var w : word);
procedure takel(var l : longint);
procedure run;
implementation
procedure take(var w : word);
begin
end;
procedure takel(var l : longint);
begin
end;
procedure run;
var
  b : array[0..15] of byte;
  i : longint;
begin
  take(unaligned(pword(@b[i])^));
  takel(longint(unaligned(plongint(@b[i])^)));
end;
end.
