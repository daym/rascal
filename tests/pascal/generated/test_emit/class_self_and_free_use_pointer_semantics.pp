unit u;
interface
type
  tnode = class
    next : tnode;
    procedure zap;
  end;
implementation
procedure tnode.zap;
begin
  self.next.free;
end;
end.
