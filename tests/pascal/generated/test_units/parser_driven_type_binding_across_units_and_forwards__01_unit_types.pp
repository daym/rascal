unit types;
interface
type
  pnode = ^tnode;
  tnode = record next : pnode; end;
  talias = tnode;
implementation
end.
