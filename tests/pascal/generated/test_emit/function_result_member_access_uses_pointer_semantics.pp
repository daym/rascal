unit u;
interface
type
  tnode = class
    next : tnode;
  end;
function clone(n : tnode) : tnode;
implementation
function clone(n : tnode) : tnode;
begin
  clone := n;
  clone.next := nil;
end;
end.
