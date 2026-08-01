unit u;
interface
function combine(a, b, c : smallint) : int64;
implementation
function combine(a, b, c : smallint) : int64;
begin combine := (a + b) * c; end;
end.
