unit u;
interface
type
  tsmall = set of 0..7;
  tbyte = set of byte;
implementation
procedure demo;
var
  small : tsmall;
begin
  if tbyte(small) = [] then begin end;
end;
end.
