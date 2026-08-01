unit u;
interface
procedure inner(var x);
procedure outer(var y);
implementation
procedure inner(var x);
begin
end;
procedure outer(var y);
begin
  inner(y);
end;
end.
