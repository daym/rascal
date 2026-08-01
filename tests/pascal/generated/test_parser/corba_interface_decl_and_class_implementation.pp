unit u;
interface
{$interfaces corba}
type
  ireader = interface ['{11111111-1111-1111-1111-111111111111}']
    function next(out s : string) : boolean;
  end;
  twriter = interface
    procedure putline(const s : string);
  end;
  treader = class(tobject, ireader, twriter)
    function next(out s : string) : boolean;
    procedure putline(const s : string);
  end;
implementation
end.
