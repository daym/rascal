unit u;
interface
type
  pint = ^longint;
  pother = ^byte;
procedure take(p : pint);
procedure run(b : pother);
implementation
procedure take(p : pint); begin end;
procedure run(b : pother);
begin
  take(pint(b));
end;
end.
