unit u;
interface
type
  trec = record
    a : longint;
    b : longint;
  end;
  prec = ^trec;
function offset(p : prec) : longint;
implementation
function offset(p : prec) : longint;
begin
  offset := ptrint(@p^.b) - ptrint(p);
end;
end.
