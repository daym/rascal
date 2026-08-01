unit u;
interface
procedure demo;
implementation
procedure prefetch(x : longint);
begin
end;
procedure demo;
var x : longint;
begin
  prefetch(x);
end;
end.
