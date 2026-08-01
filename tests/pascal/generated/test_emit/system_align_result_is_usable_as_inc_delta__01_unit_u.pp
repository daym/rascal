unit u;
interface
procedure run;
implementation
uses sizes;
procedure run;
var Offset : LongInt; Size : TSize;
begin
  Inc(Offset, Align(Sizes[Size], 4));
end;
end.
