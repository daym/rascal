unit u;
interface
procedure run;
implementation
var calls : longint;
function nextindex : longint;
begin
  inc(calls);
  nextindex := 0;
end;
procedure run;
var values : array[0..1] of longint;
begin
  inc(values[nextindex()]);
end;
end.
