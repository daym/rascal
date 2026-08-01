unit u;
interface
type
  tlist = class
  private
    fsize : longint;
    function get(index : longint) : pointer;
    procedure put(index : longint; value : pointer);
  public
    property size : longint read fsize write fsize;
    property items[index : longint] : pointer read get write put; default;
  end;
procedure demo(lst : tlist; p : pointer);
implementation
function tlist.get(index : longint) : pointer;
begin
  get := nil;
end;
procedure tlist.put(index : longint; value : pointer);
begin
end;
procedure demo(lst : tlist; p : pointer);
begin
  lst.size := 1;
  p := lst[1];
  lst[2] := p;
end;
end.
