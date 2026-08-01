unit u;
interface
type
  tbase = class end;
  tchild = class(tbase) end;
function is_child(p : tbase) : boolean;
function as_child(p : tbase) : tchild;
implementation
function is_child(p : tbase) : boolean;
begin
  is_child := p is tchild;
end;
function as_child(p : tbase) : tchild;
begin
  as_child := p as tchild;
end;
end.
