unit u;
interface
uses other;
procedure run;
procedure take(i : longint);
implementation
procedure hidden(i : longint); begin end;
procedure take(i : longint); begin end;
procedure run;
begin
  take(hidden);
end;
end.
