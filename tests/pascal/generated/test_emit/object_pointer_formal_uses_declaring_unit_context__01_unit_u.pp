unit u;
interface
uses dep;
type
  tbase = object end;
  tchild = object end;
procedure run(p : dep.pchild);
implementation
procedure run(p : dep.pchild);
begin
  dep.take(p);
end;
end.
