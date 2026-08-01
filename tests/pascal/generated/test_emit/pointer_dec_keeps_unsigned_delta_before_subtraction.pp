unit u;
interface
procedure step(var p : pchar; n : cardinal);
implementation
procedure step(var p : pchar; n : cardinal);
begin dec(p, n); end;
end.
