unit u;
interface
type
  trec = record
    value : longint;
  end;
procedure copy(var b);
implementation
procedure copy(var b);
var
  r : trec;
begin
  r := trec(b);
end;
end.
