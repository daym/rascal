unit u;
interface
type tflag = (a, b);
type tflags = set of tflag;
procedure run;
implementation
var s : tflags;
procedure take(x : longint);
begin
end;
procedure run;
begin
  take(ord(a in s));
end;
end.
