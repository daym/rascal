unit u;
interface
function bits(p : pointer) : longint;
implementation
function bits(p : pointer) : longint;
begin
  bits := longint(p);
end;
end.
