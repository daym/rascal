unit u;
interface
procedure run;
implementation
procedure run;
var
  s : shortstring;
  result : longint;
  code : integer;
begin
  val(s, cardinal(result), code);
end;
end.
