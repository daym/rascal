unit u;
interface
procedure step(var b);
implementation
procedure step(var b);
begin
  inc(longint(b));
end;
end.
