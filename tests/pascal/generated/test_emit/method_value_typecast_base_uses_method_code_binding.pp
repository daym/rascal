unit u;
interface
type
  tobject = class end;
  tcallback = procedure(p:tobject; arg:pointer) of object;
  tlist = class
    procedure foreachcall(cb : tcallback; arg : pointer);
  end;
  tbase = class
  end;
  tcasted = class(tbase)
    procedure handler(p:tobject; arg:pointer);
  end;
  thost = class
    list : tlist;
    base : tbase;
    procedure run;
  end;
implementation
procedure tlist.foreachcall(cb : tcallback; arg : pointer); begin end;
procedure tcasted.handler(p:tobject; arg:pointer); begin end;
procedure thost.run;
begin
  list.foreachcall(@tcasted(base).handler, nil);
end;
end.
