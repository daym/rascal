unit u;
interface
type
  tlist = class
    function getcount : longint;
    procedure setcount(value : longint);
    property count : longint read getcount write setcount;
  end;
procedure demo(lst : tlist);
implementation
function tlist.getcount : longint;
begin
  getcount := 0;
end;
procedure tlist.setcount(value : longint);
begin
end;
procedure demo(lst : tlist);
var
  n : longint;
begin
  n := lst.count;
  lst.count := 3;
end;
end.
