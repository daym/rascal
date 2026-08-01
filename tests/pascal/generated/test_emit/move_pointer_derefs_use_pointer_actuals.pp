unit u;
interface
procedure run(count : longint);
implementation
procedure run(count : longint);
type
  pbyte = ^byte;
var
  src, dst : pbyte;
begin
  move(src^, dst^, count);
end;
end.
