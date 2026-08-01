unit u;
interface
type
  tflag = (fa, fb, fc, fd);
function pack : longint;
implementation
function pack : longint;
begin
  pack := longint([fa, fc]);
end;
end.
