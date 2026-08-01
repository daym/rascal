unit u;
interface
procedure replace(out s : string);
procedure demo;
implementation
procedure replace(out s : string);
begin
  s := 'x';
end;
procedure demo;
var small : string[7];
begin
  replace(small);
end;
end.
