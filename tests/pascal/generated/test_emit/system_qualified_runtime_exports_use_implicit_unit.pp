unit u;
interface
procedure run;
implementation
var
  f : text;
  n, code : longint;
procedure run;
begin
  n := system.heapsize;
  system.val('12', n, code);
  system.close(f);
end;
end.
