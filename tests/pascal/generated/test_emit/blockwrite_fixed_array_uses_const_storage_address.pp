unit u;
interface
procedure run(count : longint);
implementation
procedure run(count : longint);
var
  f : file;
  buf : array[0..15] of char;
begin
  blockwrite(f, buf, count);
end;
end.
