unit u;
interface
type
  tlocation = record
    case byte of
      0 : (reg : longint);
      1 : (other : longint);
  end;
  tnode = class
    location : tlocation;
  end;
  tcall = class(tnode)
    left : tnode;
    right : tnode;
  end;
function read_reg(n : tnode) : longint;
implementation
function read_reg(n : tnode) : longint;
begin
  read_reg := tcall(tcall(n).right).left.location.reg;
end;
end.
