unit u;
interface
type
  tnode = class
    next : tnode;
  end;
procedure clear(p : pointer);
implementation
procedure clear(p : pointer);
begin
  with tnode(p) do
    next := nil;
end;
end.
