unit u;
interface
procedure outer(x : longint);
implementation
procedure outer(x : longint);
  function ready(v : longint) : boolean;
  begin
    ready := v > 0;
  end;
begin
  if (x < 10) and ready(x) then writeln(x);
end;
end.
