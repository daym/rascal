unit u;
interface
procedure demo;
implementation
type
  pbox = ^tbox;
  tbox = object
    constructor init(n : integer);
  end;
procedure demo;
var
  b : pbox;
begin
  b := new(pbox, init(3));
end;
end.
