unit u;
interface
procedure sink(const b; len : longint);
procedure demo(value : longint);
implementation
procedure sink(const b; len : longint);
begin
end;
procedure demo(value : longint);
begin
  sink(pchar(value)^, 4);
end;
end.
