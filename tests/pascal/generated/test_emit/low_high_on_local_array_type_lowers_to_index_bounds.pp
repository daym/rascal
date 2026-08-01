unit u;
interface
procedure run;
implementation
procedure run;
type localarr = array[0..7] of byte;
var i : longint;
begin
  for i := low(localarr) to high(localarr) do begin end;
end;
end.
