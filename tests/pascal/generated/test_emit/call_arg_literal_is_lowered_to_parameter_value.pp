unit u;
interface
procedure take(x : longint);
implementation
procedure take(x : longint);
begin
end;
procedure demo;
begin
  take($FFFFFFFF);
end;
end.
