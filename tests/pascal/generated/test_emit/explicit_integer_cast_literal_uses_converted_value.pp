unit u;
interface
uses sysutils;
procedure demo;
implementation
procedure demo;
var
  s : smallint;
  b : byte;
begin
  s := smallint($8000);
  b := byte(-1);
end;
end.
