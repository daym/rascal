unit u;
interface
type
  titem = class end;
  tcallback = procedure(p : titem; arg : pointer) of object;
  tlist = class
    procedure foreachcall(cb : tcallback; arg : pointer);
  end;
  tholder = class
    list : tlist;
    procedure visit(p : titem; arg : pointer);
    procedure run;
  end;
implementation
procedure tlist.foreachcall(cb : tcallback; arg : pointer); begin end;
procedure tholder.visit(p : titem; arg : pointer); begin end;
procedure tholder.run;
begin
  list.foreachcall(tcallback(@visit), nil);
end;
end.
