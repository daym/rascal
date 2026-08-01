unit u;
interface
procedure demo;
implementation
type
  plocation = ^tlocation;
  tlocation = record
    size : integer;
    case boolean of
      false : (reg : integer);
      true : (address : pointer);
  end;
var
  location : tlocation;
function add_location : plocation;
begin
  add_location := @location;
end;
procedure demo;
begin
  with add_location^ do
  begin
    size := 1;
    reg := 8;
  end;
end;
end.
