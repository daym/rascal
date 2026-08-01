unit tree;
interface
uses globtype;
type
  pdef = pointer;
  ptree = pointer;
function genrealconstnode(v : bestreal; def : pdef) : ptree;
implementation
function genrealconstnode(v : bestreal; def : pdef) : ptree;
begin
  result := nil;
end;
end.
