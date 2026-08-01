unit u;
interface
type
  tlist = class
  end;
procedure zap(list : tlist);
implementation
procedure zap(list : tlist);
begin
  list.destroy;
end;
end.
