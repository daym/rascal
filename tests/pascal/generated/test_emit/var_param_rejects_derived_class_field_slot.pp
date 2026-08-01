unit u;
interface
type
  TNode = class
  end;
  TBlock = class(TNode)
  end;
  TCall = class(TNode)
    body : TBlock;
  end;
function visit(var n : TNode) : boolean;
implementation
function visit(var n : TNode) : boolean;
begin
  result := false;
  if n is TCall then
    result := visit(TCall(n).body) or result;
end;
end.
