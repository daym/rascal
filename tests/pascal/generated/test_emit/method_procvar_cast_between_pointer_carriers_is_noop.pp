unit u;
interface
type
  titem = class end;
  titemcallback = procedure(p:titem; arg:pointer) of object;
  tlistcallback = procedure(p:pointer; arg:pointer) of object;
  tlist = class
    procedure foreachcall(cb:tlistcallback; arg:pointer);
  end;
  tobjectlist = class
    list : tlist;
    procedure foreachcall(cb:titemcallback; arg:pointer);
  end;
implementation
procedure tlist.foreachcall(cb:tlistcallback; arg:pointer); begin end;
procedure tobjectlist.foreachcall(cb:titemcallback; arg:pointer);
begin
  list.foreachcall(tlistcallback(cb), arg);
end;
end.
