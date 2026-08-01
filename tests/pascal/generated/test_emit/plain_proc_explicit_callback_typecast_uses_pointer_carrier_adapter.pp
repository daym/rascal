unit u;
interface
type
  titem = class end;
  titemcallback = procedure(p:titem; arg:pointer);
var
  cb : titemcallback;
procedure visit(p:titem; arg:pointer);
procedure run;
implementation
procedure visit(p:titem; arg:pointer); begin end;
procedure run;
begin
  cb := titemcallback(@visit);
end;
end.
