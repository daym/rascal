unit u;
interface
type
  titem = class end;
  tobjectcallback = procedure(p : titem; arg : pointer) of object;
  tstaticcallback = procedure(p : titem; arg : pointer);
  tlist = class
    procedure foreachcall(cb : tobjectcallback; arg : pointer); overload;
    procedure foreachcall(cb : tstaticcallback; arg : pointer); overload;
  end;
  tholder = class
    list : tlist;
    procedure visit(p : titem; arg : pointer);
    procedure run;
  end;
implementation
procedure tlist.foreachcall(cb : tobjectcallback; arg : pointer); begin end;
procedure tlist.foreachcall(cb : tstaticcallback; arg : pointer); begin end;
procedure tholder.visit(p : titem; arg : pointer); begin end;
procedure tholder.run;
begin
  list.foreachcall(@visit, nil);
end;
end.
