unit b;
interface
procedure run;
implementation
uses a;
type tasmtoken = (b0, b1);
var current : tasmtoken;
procedure run;
begin
  current := b0;
end;
end.
