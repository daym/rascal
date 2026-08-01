unit u;
interface
function keep(code : byte) : boolean;
implementation
function keep(code : byte) : boolean;
begin
  keep := not(code in []);
end;
end.
