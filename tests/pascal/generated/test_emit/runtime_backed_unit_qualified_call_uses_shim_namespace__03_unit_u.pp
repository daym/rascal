unit u;
interface
procedure run;
implementation
uses unix;
procedure run;
var
  p : pchar;
  rc : longint;
begin
  p := unix.getenv('PATH');
  unix.shell('true');
  rc := unix.fpsystem('true');
end;
end.
