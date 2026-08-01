unit u;
interface
type plongint = ^longint;
function demo(value : longint) : longint;
implementation
function demo(value : longint) : longint;
begin
  demo := plongint(value)^;
end;
end.
