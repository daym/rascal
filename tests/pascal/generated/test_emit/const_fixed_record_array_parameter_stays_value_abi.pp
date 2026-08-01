unit u;
interface
type
  trec = record
    x : longint;
  end;
  tarr = array[0..3] of trec;
  ptarr = ^tarr;
procedure take(const a : tarr);
implementation
procedure take(const a : tarr);
var
  p : ptarr;
begin
  p := @a;
  if a[0].x <> 0 then ;
end;
end.
