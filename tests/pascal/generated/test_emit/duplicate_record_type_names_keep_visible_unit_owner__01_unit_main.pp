unit main;
interface
uses ua, ub;
procedure run;
implementation
procedure run;
var
  x : tshared;
begin
  x.b := 1;
end;
end.
