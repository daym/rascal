unit u;
interface
type
  titem = class end;
  tinner = class
  private
    function getitem(i : longint) : titem;
  public
    property items[i : longint] : titem read getitem; default;
  end;
  touter = class
  private
    function getinner : tinner;
  public
    property inner : tinner read getinner;
  end;
function pick(o : touter; i : longint) : titem;
implementation
function tinner.getitem(i : longint) : titem;
begin
  getitem := nil;
end;
function touter.getinner : tinner;
begin
  getinner := nil;
end;
function pick(o : touter; i : longint) : titem;
begin
  pick := o.inner[i];
end;
end.
