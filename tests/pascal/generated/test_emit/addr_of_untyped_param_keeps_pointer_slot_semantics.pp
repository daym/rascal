unit u;
interface
procedure demo(var data);
implementation
procedure demo(var data);
var
  p : pchar;
begin
  p := @data;
end;
end.
