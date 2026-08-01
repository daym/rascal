unit u;
interface
type pbyte = ^byte;
procedure run;
implementation
procedure run;
var c : char; p : pbyte;
begin
  c := 'A';
  inc(ord(c));
  ord(c) := 66;
  p := @ord(c);
end;
end.
