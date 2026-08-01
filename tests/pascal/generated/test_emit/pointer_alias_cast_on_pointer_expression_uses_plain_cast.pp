unit u;
interface
procedure demo;
implementation
procedure demo;
type
  plongint = ^longint;
var
  raw : array[0..7] of byte;
  i : longint;
  l : longint;
begin
  l := plongint(@raw[i*4])^;
end;
end.
