unit tcadd;
interface
uses tree;
procedure run;
implementation
uses globtype;
procedure run;
var
  lvd, rvd : bestreal;
  def : pdef;
  t : ptree;
begin
  t := genrealconstnode(lvd/rvd, def);
  t := genrealconstnode(1.0, def);
end;
end.
