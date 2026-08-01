unit u;
interface
type
  tbase = class end;
  tchild = class(tbase) end;
function cast_child(p : tbase) : tchild;
implementation
function cast_child(p : tbase) : tchild;
begin
  cast_child := tchild(p);
end;
end.
