unit u;
interface
function f(a, b : longint) : boolean;
implementation
function f(a, b : longint) : boolean;
begin
  f := (a = 1) and (b = 2) and not ((a = 3) xor (b = 4));
end;
end.
