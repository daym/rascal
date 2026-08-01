unit u;
interface
type
  titems = class
    function get(index : longint) : pointer;
    procedure put(index : longint; value : pointer);
    property items[index : longint] : pointer read get write put; default;
  end;
  tbox = class
  private
    flist : titems;
  public
    property list : titems read flist;
  end;
procedure demo(box : tbox; p : pointer);
implementation
function titems.get(index : longint) : pointer;
begin
  get := nil;
end;
procedure titems.put(index : longint; value : pointer);
begin
end;
procedure demo(box : tbox; p : pointer);
begin
  p := box.list[1];
  box.list[2] := p;
end;
end.
