unit u;
interface
type trec = record x : longint; end;
procedure take(constref r : trec);
procedure keep(constref : trec);
implementation
end.
