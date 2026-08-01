unit u;
interface
procedure demo;
implementation
procedure demo;
type
  pdata = ^tdata;
  tdata = record
    x, y : integer;
  end;
var
  d : pdata;
begin
  new(d);
  with d^ do
  begin
    x := 1;
    y := x;
  end;
end;
end.
