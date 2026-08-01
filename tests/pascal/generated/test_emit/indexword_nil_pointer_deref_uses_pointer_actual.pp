unit u;
interface
procedure run;
implementation
procedure run;
type
  psuper = ^word;
var
  buf : psuper;
  len : longint;
  s : word;
begin
  if indexword(buf^, len, s) = -1 then ;
end;
end.
