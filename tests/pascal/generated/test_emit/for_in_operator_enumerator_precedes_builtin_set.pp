unit u;
interface
type
  tregs = set of 0..3;
  tenumerator = class
    fcurrent : integer;
    function MoveNext : boolean;
    property Current : integer read fcurrent;
  end;
procedure p;
implementation
operator enumerator(s : tregs) : tenumerator;
begin
  Result := nil;
end;
function tenumerator.MoveNext : boolean;
begin
  Result := false;
end;
procedure p;
var regs : tregs; j : integer;
begin
  for j in regs do
    j := j + 1;
end;
end.
