unit u;
interface
type
  tnode = class end;
  thost = class
    function callit:tnode;
  end;
implementation
function thost.callit:tnode;
type
  tprocedureofobject = function:tnode of object;
var
  m : TMethod;
begin
  m.Code := nil;
  m.Data := self;
  if assigned(m.Code) then
    result := tprocedureofobject(m)();
end;
end.
