unit u;
interface
type
  trec = record
    state : (idle, busy);
  end;
var r : trec;
procedure run;
implementation
procedure run;
begin
  r.state := busy;
end;
end.
