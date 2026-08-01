unit u;
interface
type
  tsym = class
    name : string;
  end;
  tblock = class
    sym : tsym;
  end;
function getname(b : tblock) : string;
implementation
function getname(b : tblock) : string;
begin
  getname := b.sym.name;
end;
end.
