unit u;
interface
type
  tbox = record
    v : longint;
  end;
  tcarrier = object
    fvalue : longint;
    property value : longint read fvalue;
    function add(x : longint) : longint;
  end;
operator + (const a,b : tbox) : tcarrier;
procedure demo(a,b : tbox; var i : longint);
implementation
operator + (const a,b : tbox) : tcarrier;
begin
  result.fvalue := a.v + b.v;
end;
function tcarrier.add(x : longint) : longint;
begin
  add := fvalue + x;
end;
procedure demo(a,b : tbox; var i : longint);
begin
  i := (a + b).value + (a + b).add(2);
end;
end.
