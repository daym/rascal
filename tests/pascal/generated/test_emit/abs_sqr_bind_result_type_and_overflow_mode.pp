unit u;
interface
type tdistinct = type smallint;
procedure run(si : smallint; w : word; lw : longword; d : tdistinct);
implementation
procedure run(si : smallint; w : word; lw : longword; d : tdistinct);
var a, s : int64; q : qword;
begin
{$Q-}
  a := abs(si);
  a := abs(d);
  s := sqr(w);
  q := sqr(lw);
{$Q+}
  s := sqr(si);
end;
end.
