unit u;
interface
type
  tenumerator = class
    fcurrent : integer;
    function MoveNext : boolean;
    property Current : integer read fcurrent;
  end;
  tbox = class
    function GetEnumerator : tenumerator;
  end;
procedure p;
implementation
function tenumerator.MoveNext : boolean;
begin
  Result := false;
end;
function tbox.GetEnumerator : tenumerator;
begin
  Result := nil;
end;
procedure p;
var box : tbox; i : integer;
begin
  for i in box do
    i := i + 1;
end;
end.
