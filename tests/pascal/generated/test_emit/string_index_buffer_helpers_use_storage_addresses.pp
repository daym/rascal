unit u;
interface
procedure run(i, count : longint);
implementation
procedure run(i, count : longint);
var
  s1, s2 : string;
begin
  if comparechar(s1[i], s2[i], count) = 0 then ;
  if indexbyte(s1[i], count, byte('c')) >= 0 then ;
end;
end.
