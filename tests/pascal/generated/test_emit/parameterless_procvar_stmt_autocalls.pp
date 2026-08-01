unit u;
interface
type
  tstop = procedure;
var
  oldstop : tstop;
procedure kick;
implementation
procedure kick;
begin
  oldstop;
end;
end.
