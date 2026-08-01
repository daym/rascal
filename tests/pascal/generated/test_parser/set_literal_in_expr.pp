program p;
type tcolor = (r,g,b);
var c : tcolor;
begin
  if c in [r, g] then c := b;
end.
