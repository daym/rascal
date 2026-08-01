unit u;
interface
procedure demo;
implementation
procedure demo;
type
  ppchar = ^pchar;
var
  raw : pointer;
  p : ppchar;
begin
  p := pointer(raw);
end;
end.
