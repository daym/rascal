unit u;
interface
type tpred = function(var s : string) : boolean;
procedure demo(p : tpred);
implementation
procedure demo(p : tpred);
var value : string; b : boolean;
begin
  b := p(value);
end;
end.
