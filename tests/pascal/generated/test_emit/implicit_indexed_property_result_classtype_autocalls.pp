unit u;
interface
type
  titem = class
  end;
  tbox = class
  private
    function getitem(i : integer) : titem;
  public
    property items[i : integer] : titem read getitem; default;
    function sameclass(i : integer; c : tclass) : boolean;
  end;
implementation
function tbox.getitem(i : integer) : titem;
begin
  getitem := nil;
end;
function tbox.sameclass(i : integer; c : tclass) : boolean;
begin
  sameclass := items[i].classtype = c;
end;
end.
