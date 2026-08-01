unit u;
interface
type trec = packed record name : pchar; end;
procedure run;
implementation
procedure run;
var
  r : trec;
  p : pchar;
begin
  p := unaligned(r.name);
end;
end.
