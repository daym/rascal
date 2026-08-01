unit u;
interface
type
  tpred = function : boolean;
var
  pred : tpred;
function go : boolean;
implementation
function go : boolean;
begin
  go := pred() and true;
end;
end.
