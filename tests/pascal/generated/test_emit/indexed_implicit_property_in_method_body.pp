unit u;
interface
type
  tlist = class
  private
    function get(i : integer) : integer;
  public
    function first : integer;
    property items[i : integer] : integer read get; default;
  end;
implementation
function tlist.get(i : integer) : integer;
begin
  get := i;
end;
function tlist.first : integer;
begin
  first := items[0];
end;
end.
