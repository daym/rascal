unit a;
interface
procedure touch;
implementation
type tasmtoken = (a0, a1);
var current : tasmtoken;
procedure touch;
begin
  current := a0;
end;
end.
