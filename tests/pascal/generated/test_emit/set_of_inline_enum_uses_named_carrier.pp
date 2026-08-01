unit u;
interface
type
  tflags = set of (red, green, blue);
var flags : tflags;
procedure run;
implementation
procedure run;
begin
  flags := [red, blue];
end;
end.
