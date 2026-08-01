unit listunit;
interface
type
  tfplist = class
  private
    fcount : longint;
  public
    function getcount : longint;
    property count : longint read fcount;
  end;
implementation
function tfplist.getcount : longint;
begin
  getcount := fcount;
end;
end.
