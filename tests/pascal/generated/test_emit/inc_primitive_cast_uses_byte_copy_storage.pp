unit u;
interface
procedure step(p : pchar);
implementation
procedure step(p : pchar);
begin
  inc(longint(p));
end;
end.
