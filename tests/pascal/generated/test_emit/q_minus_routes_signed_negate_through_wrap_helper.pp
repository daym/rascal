unit u;
interface
function neg_q(v : int64) : int64;
function neg_n(v : int64) : int64;
implementation
{$Q+}
function neg_q(v : int64) : int64;
begin
  neg_q := -v;
end;
{$Q-}
function neg_n(v : int64) : int64;
begin
  neg_n := -v;
end;
end.
