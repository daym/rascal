unit u;
interface
type
  tnum = record
    v : longint;
  end;
  tenumerator = class
    fcurrent : integer;
    function MoveNext : boolean;
    property Current : integer read fcurrent;
  end;
  tbox = class
    function GetEnumerator : tenumerator;
  end;
operator + (const a,b : tnum) : tbox;
procedure p;
implementation
operator + (const a,b : tnum) : tbox;
begin
  result := nil;
end;
function tenumerator.MoveNext : boolean;
begin
  Result := false;
end;
function tbox.GetEnumerator : tenumerator;
begin
  Result := nil;
end;
procedure p;
var a,b : tnum; i : integer;
begin
  for i in a + b do
    i := i + 1;
end;
end.
