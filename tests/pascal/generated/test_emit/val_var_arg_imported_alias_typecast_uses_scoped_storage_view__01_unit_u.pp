unit u;
interface
uses globtype;
procedure run;
implementation
procedure run;
var
  s : shortstring;
  result : longint;
  code : integer;
begin
  val(s, aword(result), code);
end;
end.
