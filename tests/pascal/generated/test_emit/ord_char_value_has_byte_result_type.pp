unit u;
interface
procedure run(c : char);
implementation
function pick(x : byte) : byte;
begin
  pick := x;
end;
function pick(x : longint) : byte;
begin
  pick := 0;
end;
procedure run(c : char);
var b : byte;
begin
  b := pick(ord(c));
end;
end.
