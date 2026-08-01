unit u;
interface
function run(p : int64) : int64;
implementation
function run(p : int64) : int64;
begin
  if p > system.high(int64) div 2 then run := 0;
  if p < system.low(int64) div 2 then run := 0;
end;
end.
