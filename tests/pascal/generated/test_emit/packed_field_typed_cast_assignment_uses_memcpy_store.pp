unit u;
interface
type tg = packed record d1 : longword; end;
procedure run(var p : tg; v : longint);
implementation
procedure run(var p : tg; v : longint);
begin
  longint(p.d1) := v;
end;
end.
