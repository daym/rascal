unit u;
interface
procedure inner(const x; len : longint);
procedure outer(const y; len : longint);
implementation
procedure inner(const x; len : longint);
begin
end;
procedure outer(const y; len : longint);
begin
  inner(y, len);
end;
end.
