unit u;
interface
type
  titem = class end;
  taggregate = class
  public type
    tenumerator = class
    private
      fcurrent : titem;
    public
      constructor Create(data : taggregate);
      function MoveNext : boolean;
      property Current : titem read fcurrent;
    end;
  protected
    fcount : longint;
  public
    function GetEnumerator : tenumerator;
    procedure Convert;
  end;
implementation
constructor taggregate.tenumerator.Create(data : taggregate);
begin
  inherited Create;
end;
function taggregate.tenumerator.MoveNext : boolean;
begin
  Result := false;
end;
function taggregate.GetEnumerator : tenumerator;
begin
  Result := tenumerator.Create(self);
end;
procedure taggregate.Convert;
var item : titem;
begin
  for item in self do
    item := nil;
end;
end.
