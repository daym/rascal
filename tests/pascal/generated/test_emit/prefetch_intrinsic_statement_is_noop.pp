unit u;
interface
procedure demo;
implementation
procedure demo;
var x : longint; p : ^longint;
begin
  prefetch(x);
  system.prefetch(p^);
  x := 1;
end;
end.
