unit u;
interface
type
  titem = class end;
  titemcallback = procedure(p:titem; arg:pointer);
  tlistcallback = procedure(p:pointer; arg:pointer);
var
  cb : titemcallback;
  listcb : tlistcallback;
procedure visit(p:titem; arg:pointer);
procedure run;
implementation
procedure visit(p:titem; arg:pointer); begin end;
procedure run;
begin
  cb := @visit;
  listcb := tlistcallback(cb);
end;
end.
