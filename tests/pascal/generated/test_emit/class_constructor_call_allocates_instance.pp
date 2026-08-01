unit u;
interface
type
  tnode = class
    constructor create;
  end;
function build : tnode;
implementation
constructor tnode.create;
begin
end;
function build : tnode;
begin
  build := tnode.create;
end;
end.
