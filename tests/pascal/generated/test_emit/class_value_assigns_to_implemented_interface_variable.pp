unit u;
interface
{$interfaces corba}
type
  ireader = interface ['{11111111-1111-1111-1111-111111111111}']
    procedure next;
  end;
  treader = class(tobject, ireader)
    procedure next;
  end;
procedure run(r : treader);
implementation
procedure treader.next;
begin
end;
procedure run(r : treader);
var i : ireader;
begin
  i := r;
end;
end.
