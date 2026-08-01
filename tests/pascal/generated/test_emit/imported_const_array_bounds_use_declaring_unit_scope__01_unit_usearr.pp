unit usearr;
interface
uses arrunit;
procedure setoper(var r : trec; i : longint);
implementation
procedure setoper(var r : trec; i : longint);
begin
  r.oper[i] := 1;
end;
end.
