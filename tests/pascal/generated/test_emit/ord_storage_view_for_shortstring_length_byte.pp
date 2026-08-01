unit u;
interface
procedure run;
implementation
procedure run;
var s : string[10];
begin
  s := 'abc';
  dec(ord(s[0]));
end;
end.
