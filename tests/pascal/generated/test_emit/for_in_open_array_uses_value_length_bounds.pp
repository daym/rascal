unit u;
interface
type
  tasmop = (a_none, a_add);
function p(const ops : array of tasmop) : boolean;
implementation
function p(const ops : array of tasmop) : boolean;
var op : tasmop;
begin
  Result := false;
  for op in ops do
    Result := true;
end;
end.
