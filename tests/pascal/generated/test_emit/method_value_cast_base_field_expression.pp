unit u;
interface
type
  tobject = class end;
  tcallback = procedure(p:tobject; arg:pointer) of object;
  tstoredsymtable = class
    procedure testfordefaultproperty(p:tobject; arg:pointer);
  end;
  tlist = class
    procedure foreachcall(cb : tcallback; arg : pointer);
  end;
  thelp = class
    symtable : tstoredsymtable;
  end;
var
  helperpd : thelp;
implementation
procedure tlist.foreachcall(cb : tcallback; arg : pointer); begin end;
procedure tstoredsymtable.testfordefaultproperty(p:tobject; arg:pointer); begin end;
procedure demo;
var
  list : tlist;
  host : thelp;
begin
  list.foreachcall(@tstoredsymtable(host.symtable).testfordefaultproperty, nil);
end;
end.
