unit u;
interface
type
  titem = class end;
  tlist = class
  private
    function getitem(i : longint) : titem;
  public
    property items[i : longint] : titem read getitem; default;
  end;
function pick(l : tlist; i : longint) : titem;
implementation
function tlist.getitem(i : longint) : titem;
begin
  getitem := nil;
end;
function pick(l : tlist; i : longint) : titem;
begin
  pick := l[i];
end;
end.
