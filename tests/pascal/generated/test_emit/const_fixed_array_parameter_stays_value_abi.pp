unit u;
interface
type
  tarr = array[0..3] of longint;
  ptarr = ^tarr;
procedure take(const a : tarr);
implementation
procedure take(const a : tarr);
var
  p : ptarr;
begin
  p := @a;
  if a[0] <> 0 then ;
end;
end.
