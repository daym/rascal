unit u;
interface
function checked(a, b : longint) : longint;
function wrapped(a, b : longint) : longint;
implementation
{$Q+}
function checked(a, b : longint) : longint;
begin checked := a div b; end;
{$Q-}
function wrapped(a, b : longint) : longint;
begin wrapped := a div b; end;
end.
