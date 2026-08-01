unit u;
interface
type
  titem = class end;
  tlistcallback = procedure(p:pointer; arg:pointer);
  tlist = class
    procedure foreachcall(cb:tlistcallback; arg:pointer);
  end;
procedure visit(p:titem; arg:pointer);
procedure run(list:tlist);
implementation
procedure tlist.foreachcall(cb:tlistcallback; arg:pointer); begin end;
procedure visit(p:titem; arg:pointer); begin end;
procedure run(list:tlist);
begin
  list.foreachcall(@visit, nil);
end;
end.
