unit u;
interface
{$H+}
function demo(p : pchar; i : longint) : string;
implementation
function demo(p : pchar; i : longint) : string;
begin
  demo := string(PChar(@p[i]));
end;
end.
