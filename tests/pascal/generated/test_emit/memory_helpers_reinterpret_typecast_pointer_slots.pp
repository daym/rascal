unit u;
interface
procedure demo;
implementation
type
  pdata = ^byte;
procedure demo;
var
  raw : pointer;
begin
  getmem(pdata(raw), 4);
  freemem(pdata(raw), 4);
  dispose(pdata(raw));
end;
end.
