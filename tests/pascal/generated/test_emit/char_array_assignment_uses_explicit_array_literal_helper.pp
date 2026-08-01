unit u;
interface
procedure setbuf;
implementation
procedure setbuf;
var
  buf : array[1..4] of char;
begin
  buf := 'A';
end;
end.
