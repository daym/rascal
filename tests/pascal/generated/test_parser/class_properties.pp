unit u;
interface
type
  tlist = class
  private
    fcount : integer;
    function get(index : integer) : pointer;
    procedure put(index : integer; value : pointer);
  public
    property Count : integer read fcount;
    property Items[index : integer] : pointer read get write put; default;
  end;
implementation
end.
