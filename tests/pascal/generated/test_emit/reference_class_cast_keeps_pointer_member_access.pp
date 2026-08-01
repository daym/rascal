unit u;
interface
type
  tbase = class end;
  tchild = class(tbase)
    next : tchild;
  end;
function fetch_next(p : tbase) : tchild;
implementation
function fetch_next(p : tbase) : tchild;
begin
  fetch_next := tchild(p).next;
end;
end.
