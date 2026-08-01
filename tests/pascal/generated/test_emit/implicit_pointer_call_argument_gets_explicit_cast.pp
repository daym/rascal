unit u;
interface
type
  pint = ^longint;
procedure take(p : pint);
procedure demo(raw : pointer);
implementation
procedure take(p : pint);
begin
end;
procedure demo(raw : pointer);
begin
  take(raw);
end;
end.
