unit u;
interface
procedure demo;
implementation
type
  pdata = ^tdata;
  tdata = record
    value : integer;
  end;
procedure demo;
var
  d : pdata;
begin
  new(d);
  dispose(d);
end;
end.
