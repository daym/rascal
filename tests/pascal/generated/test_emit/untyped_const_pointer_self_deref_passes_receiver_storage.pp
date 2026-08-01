unit u;
interface
type
  tobj = class
    function copy : tobj;
  end;
procedure move(const src; var dst; len : longint);
implementation
procedure move(const src; var dst; len : longint);
begin
end;
function tobj.copy : tobj;
var p : tobj;
begin
  p := nil;
  move(pointer(self)^, pointer(p)^, 0);
  copy := p;
end;
end.
