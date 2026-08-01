unit u;
interface
function run : longint;
implementation
function new(value : longint) : longint;
begin
  new := value;
end;
function run : longint;
begin
  run := new(7);
end;
end.
