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
  end;
function sameclass(b : tbox; i : integer; c : tclass) : boolean;
implementation
function tbox.getitem(i : integer) : titem;
begin
  getitem := nil;
end;
function sameclass(b : tbox; i : integer; c : tclass) : boolean;
begin
  sameclass := b.items[i].classtype = c;
end;
end.
