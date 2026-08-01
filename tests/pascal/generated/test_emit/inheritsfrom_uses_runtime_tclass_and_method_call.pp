unit u;
interface
type
  tbase = class
  end;
  tchild = class(tbase)
  end;
function isbase(x : tbase; c : tclass) : boolean;
implementation
function isbase(x : tbase; c : tclass) : boolean;
begin
  isbase := x.inheritsfrom(c);
end.
