unit u;
interface
procedure take(const s : string);
procedure demo(p : pchar);
implementation
procedure take(const s : string);
begin
end;
procedure demo(p : pchar);
begin
  take(pchar(p));
end;
end.
