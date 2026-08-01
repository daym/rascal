unit u;
interface
procedure scale(var c : currency);
procedure single_div(a, b : single; var c : single);
procedure extended_div(a, b : extended; var c : extended);
implementation
procedure scale(var c : currency);
begin
  c := c / 10000;
end;
procedure single_div(a, b : single; var c : single);
begin
  c := a / b;
end;
procedure extended_div(a, b : extended; var c : extended);
begin
  c := a / b;
end;
end.
