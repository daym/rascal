unit u;
interface
type
  titem = class end;
  taggregate = class
  public
    type tenumerator = class
      fcurrent : titem;
      function MoveNext : boolean;
      property Current : titem read fcurrent;
    end;
    function GetEnumerator : tenumerator;
  end;
procedure p(agg : taggregate);
implementation
function taggregate.tenumerator.MoveNext : boolean;
begin
  Result := false;
end;
function taggregate.GetEnumerator : tenumerator;
begin
  Result := nil;
end;
procedure p(agg : taggregate);
var item : titem;
begin
  for item in agg do
    item := nil;
end;
end.
