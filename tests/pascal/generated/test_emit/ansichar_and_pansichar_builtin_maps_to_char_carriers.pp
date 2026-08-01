unit u;
interface
type
  ta = ansichar;
  tpa = pansichar;
  tbuf = array[1..3] of ansichar;
var c : ansichar; p : pansichar; buf : tbuf;
procedure take(c : ansichar; p : pansichar);
implementation
procedure take(c : ansichar; p : pansichar);
begin
end;
procedure demo;
begin
  c := ansichar(65);
  p := pansichar(@c);
  c := p^;
  buf := 'ab';
end;
end.
