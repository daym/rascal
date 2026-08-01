unit u;
interface
procedure take(const s : string);
procedure run(i : longint);
implementation
procedure take(const s : string);
begin
end;
procedure run(i : longint);
var
  s : string;
begin
  take(s[i]);
end;
end.
