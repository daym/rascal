unit u;
interface
procedure demo(a, b : longint; var q, r : longint);
implementation
procedure demo(a, b : longint; var q, r : longint);
begin
  q := a div b;
  r := a mod b;
end;
end.
