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
  r : packed record
    proc : pointer;
    obj : pointer;
  end;
begin
  r.proc := nil;
  r.obj := self;
  if assigned(r.proc) then
    result := tprocedureofobject(r)();
end;
end.
