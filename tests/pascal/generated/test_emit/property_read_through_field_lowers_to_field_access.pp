unit u;
interface
type
  tbox = class
  private
    fval : longint;
  public
    property val : longint read fval write fval;
  end;
function read_it(b : tbox) : longint;
implementation
function read_it(b : tbox) : longint;
begin
  read_it := b.val;
end;
end.
