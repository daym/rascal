unit u;
interface
type
  tbase = class
  end;
  tchild = class(tbase)
  end;
function ischild(x : tbase) : boolean;
implementation
function ischild(x : tbase) : boolean;
begin
  ischild := x.classtype = tchild;
end;
end.
