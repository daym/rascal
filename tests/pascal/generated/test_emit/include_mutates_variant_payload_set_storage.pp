unit u;
interface
type
  tkind = (red, green);
  tkinds = set of tkind;
  tbox = record
    case tag : longint of
      0 : (items : tkinds);
      1 : (value : longint);
  end;
procedure mark(var b : tbox);
implementation
procedure mark(var b : tbox);
begin
  include(b.items, red);
end;
end.
