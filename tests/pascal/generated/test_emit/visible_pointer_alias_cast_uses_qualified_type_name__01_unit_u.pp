unit u;
interface
uses widestr;
procedure demo(raw : pointer);
implementation
procedure demo(raw : pointer);
begin
  widestr.copywidestring(widestr.pcompilerwidestring(raw),
                         widestr.pcompilerwidestring(raw));
end;
end.
