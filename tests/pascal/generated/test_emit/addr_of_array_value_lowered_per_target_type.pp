unit u;
interface
type
  tbuf = array[0..3] of char;
  pbuf = ^tbuf;
procedure demo;
implementation
procedure demo;
var
  buf : tbuf;
  pc : pchar;
  pa : pbuf;
begin
  pc := @buf;
  pa := @buf;
end;
end.
