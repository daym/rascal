unit u;
interface
procedure run(i, count : longint);
implementation
procedure run(i, count : longint);
var
  f : file;
  s : string;
  transferred : longint;
begin
  blockwrite(f, s[i], count, transferred);
  blockread(f, s[i], count, transferred);
end;
end.
