unit u;
interface
type
  titem = class end;
  titemcallback = procedure(p:titem; arg:pointer);
  tlistcallback = procedure(p:pointer; arg:pointer);
procedure visit(p:titem; arg:pointer);
procedure take(cb:titemcallback); overload;
procedure take(cb:tlistcallback); overload;
procedure run;
implementation
procedure visit(p:titem; arg:pointer); begin end;
procedure take(cb:titemcallback); begin end;
procedure take(cb:tlistcallback); begin end;
procedure run;
begin
  take(@visit);
end;
end.
