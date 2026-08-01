unit u;
interface
type
  tnode = class
    constructor create(n : integer = 7);
  end;
function build : tnode;
implementation
constructor tnode.create(n : integer);
begin
end;
function build : tnode;
begin
  build := tnode.create;
end;
end.
