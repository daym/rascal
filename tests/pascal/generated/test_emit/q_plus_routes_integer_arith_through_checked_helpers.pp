unit u;
interface
function add_q(a, b : longint) : longint;
function add_n(a, b : longint) : longint;
implementation
{$Q+}
function add_q(a, b : longint) : longint;
begin
  add_q := a + b;
end;
{$Q-}
function add_n(a, b : longint) : longint;
begin
  add_n := a + b;
end;
end.
