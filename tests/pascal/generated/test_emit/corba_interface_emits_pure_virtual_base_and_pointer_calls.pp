unit u;
interface
{$interfaces corba}
type
  ireader = interface ['{11111111-1111-1111-1111-111111111111}']
    function next(out s : string) : boolean;
  end;
  treader = class(tobject, ireader)
    function next(out s : string) : boolean;
  end;
procedure use(reader : ireader);
implementation
function treader.next(out s : string) : boolean;
begin
  next := false;
end;
procedure use(reader : ireader);
var s : string;
begin
  reader.next(s);
end;
end.
