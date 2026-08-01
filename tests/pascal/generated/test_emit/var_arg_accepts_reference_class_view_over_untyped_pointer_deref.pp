unit u;
interface
type
  tnode = class
  end;
  tstatementnode = class(tnode)
  end;
procedure addstatement(var statement : tstatementnode; node : tnode);
procedure callback(arg : pointer; node : tnode);
implementation
procedure addstatement(var statement : tstatementnode; node : tnode);
begin
end;
procedure callback(arg : pointer; node : tnode);
begin
  addstatement(tstatementnode(arg^), node);
end;
end.
