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
  n : longint;
  b : boolean;
begin
  b := pc > @buf;
  pc := @buf + 1;
  b := pc > @buf + n;
  b := @buf + n > pc;
  pa := @buf;
end;
end.
