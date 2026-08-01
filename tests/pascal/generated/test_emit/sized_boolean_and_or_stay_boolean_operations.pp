unit u;
interface
procedure combine(a, b : bytebool; var both, either : bytebool);
implementation
procedure combine(a, b : bytebool; var both, either : bytebool);
begin
  both := a and b;
  either := a or b;
end;
end.
