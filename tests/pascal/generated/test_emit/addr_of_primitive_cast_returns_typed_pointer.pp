unit u;
interface
type
  plongint = ^longint;
function addr(var b) : plongint;
implementation
function addr(var b) : plongint;
begin
  addr := @longint(b);
end;
end.
