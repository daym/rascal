unit u;
interface
procedure run;
implementation
uses linux;
procedure run;
var
  p : pchar;
begin
  p := linux.getenv('PATH');
  linux.shell('true');
end;
end.
