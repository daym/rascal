unit u;
interface
type
  tdata = record
    typ : longint;
  end;
  tbox = class
    data : tdata;
    property datatype : longint read data.typ write data.typ;
  end;
function read_it(b : tbox) : longint;
procedure write_it(b : tbox);
implementation
function read_it(b : tbox) : longint;
begin
  read_it := b.datatype;
end;
procedure write_it(b : tbox);
begin
  b.datatype := 9;
end;
end.
