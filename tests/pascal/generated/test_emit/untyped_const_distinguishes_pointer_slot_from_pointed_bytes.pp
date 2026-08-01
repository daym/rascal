unit u;
interface
procedure sink(const b; len : longint);
procedure demo;
implementation
procedure sink(const b; len : longint);
begin
end;
procedure demo;
var
  p : pchar;
begin
  sink(p, 1);
  sink(p^, 1);
end;
end.
