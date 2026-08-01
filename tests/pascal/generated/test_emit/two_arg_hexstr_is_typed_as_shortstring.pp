unit u;
interface
procedure message1(w : longint; const s1 : string);
procedure demo;
implementation
procedure message1(w : longint; const s1 : string);
begin
end;
procedure demo;
begin
  message1(1, hexstr(42, 8));
end;
end.
