unit u;
interface
procedure demo(i : longint);
implementation
procedure demo(i : longint);
type
  leftright = (left, right);
var
  lr : leftright;
begin
  if i = 0 then
    lr := right
  else
    lr := left;
  writeln(ord(lr));
end;
end.
