unit u;
interface
procedure run;
implementation
procedure run;
const
  maxlevel = 16;
var
  skip : array[0..maxlevel-1] of boolean;
begin
  skip[0] := true;
end;
end.
