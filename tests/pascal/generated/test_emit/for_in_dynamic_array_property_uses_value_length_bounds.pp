unit u;
interface
type
  tsegmentlist = array of integer;
  tgroup = class
    fsegmentlist : tsegmentlist;
    property SegmentList : tsegmentlist read fsegmentlist;
  end;
procedure p(g : tgroup);
implementation
procedure p(g : tgroup);
var segment : integer; total : integer;
begin
  total := 0;
  for segment in g.SegmentList do
    total := total + segment;
end;
end.
