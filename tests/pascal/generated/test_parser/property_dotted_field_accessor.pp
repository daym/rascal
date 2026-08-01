unit u;
interface
type
  tdata = record
    typ : integer;
  end;
  tbox = class
    data : tdata;
    property datatype : integer read data.typ write data.typ;
  end;
implementation
end.
