unit u;
interface
procedure demo(const xs : array of longint);
implementation
procedure demo(const xs : array of longint);
begin
  if high(xs) = 1 then begin end;
  if low(xs) = 0 then begin end;
end;
end.
