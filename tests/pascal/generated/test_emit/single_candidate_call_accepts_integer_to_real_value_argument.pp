unit u;
interface
type bestreal = extended;
procedure take(r : bestreal);
procedure run(i : longint);
implementation
procedure take(r : bestreal); begin end;
procedure run(i : longint);
begin
  take(0);
  take(i);
end;
end.
