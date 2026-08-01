unit u;
interface
type
  tnode = class
    next : tnode;
    destructor destroy; override;
  end;
var
  head : tnode;
implementation
end.
