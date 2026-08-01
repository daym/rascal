unit useops;
interface
uses ops;
procedure test;
implementation
procedure test;
var b : tbox;
begin
  b := qword(-1);
  b := $ffffffffffffffff;
end;
end.
