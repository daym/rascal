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
    procedure demo(p : pointer);
    property list : titems read flist;
  end;
implementation
function titems.get(index : longint) : pointer;
begin
  get := nil;
end;
procedure titems.put(index : longint; value : pointer);
begin
end;
procedure tbox.demo(p : pointer);
begin
  p := list[1];
  list[2] := p;
end;
end.
