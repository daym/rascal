unit u;
interface
type
  tbase = class
  end;
function isbase(x : tbase; c : tclass) : boolean;
implementation
function isbase(x : tbase; c : tclass) : boolean;
begin
  if x.inheritsfrom(c) and assigned(x) then
    isbase := true
  else
    isbase := false;
end.
