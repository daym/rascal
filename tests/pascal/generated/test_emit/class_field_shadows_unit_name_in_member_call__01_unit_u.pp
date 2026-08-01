unit u;
interface
uses symtable;
type
  titem = class end;
  tcb = procedure(p:titem; arg:pointer) of object;
  tcontainer = class
    procedure foreach(cb : tcb; arg : pointer);
  end;
  tabstractrec = class
    symtable : tcontainer;
    procedure handler(p:titem; arg:pointer);
  end;
  tderived = class(tabstractrec)
    procedure run;
  end;
implementation
procedure tcontainer.foreach(cb : tcb; arg : pointer); begin end;
procedure tabstractrec.handler(p:titem; arg:pointer); begin end;
procedure tderived.run;
begin
  symtable.foreach(@handler, nil);
end;
end.
