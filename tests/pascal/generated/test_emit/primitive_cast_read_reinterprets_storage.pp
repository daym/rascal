unit u;
interface
procedure fetch(var b);
implementation
procedure fetch(var b);
var
  l : longint;
begin
  l := longint(b);
end;
end.
