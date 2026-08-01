unit u;
interface
type
  tnode = class
    constructor create_internal(n : integer);
  end;
  tnodeclass = class of tnode;
var
  cls : tnodeclass;
  inst : tnode;
implementation
constructor tnode.create_internal(n : integer);
begin
end;
begin
  inst := cls.create_internal(7);
end.
