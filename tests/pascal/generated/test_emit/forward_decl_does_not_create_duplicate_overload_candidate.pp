unit u;
interface
procedure run;
implementation
function statement : longint; forward;
procedure run;
var i : longint;
begin
  i := statement();
end;
function statement : longint;
begin
  statement := 0;
end;
end.
