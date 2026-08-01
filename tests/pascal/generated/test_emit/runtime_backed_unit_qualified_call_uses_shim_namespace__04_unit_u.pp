unit u;
interface
procedure run;
implementation
uses baseunix;
procedure run;
var
  p : pchar;
  rc : longint;
begin
  p := baseunix.fpgetenv('PATH');
  rc := fpchmod('x', 493);
end;
end.
