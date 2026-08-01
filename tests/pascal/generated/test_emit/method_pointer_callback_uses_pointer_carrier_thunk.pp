unit u;
interface
type
  titem = class end;
  tcallback = procedure(p:titem; arg:pointer) of object;
  thost = class
    cb : tcallback;
    procedure visit(p:titem; arg:pointer);
    procedure run;
  end;
implementation
procedure thost.visit(p:titem; arg:pointer); begin end;
procedure thost.run;
begin
  cb := @visit;
end;
end.
