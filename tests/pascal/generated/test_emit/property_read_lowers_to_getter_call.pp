unit u;
interface
type
  tbox = class
  private
    function getval : longint;
  public
    property val : longint read getval;
  end;
function read_it(b : tbox) : longint;
implementation
function tbox.getval : longint;
begin
  getval := 0;
end;
function read_it(b : tbox) : longint;
begin
  read_it := b.val;
end;
end.
