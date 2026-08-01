unit u;
interface
type
  tnode = class
  end;
  pnode = ^tnode;
  tlist = class
    function getitem(i : longint) : pointer;
    property items[i : longint] : pointer read getitem; default;
  end;
procedure replace(list : tlist; i : longint; node : tnode);
implementation
procedure replace(list : tlist; i : longint; node : tnode);
begin
  pnode(list[i])^ := node;
  node := pnode(list[i])^;
end;
end.
