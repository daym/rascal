unit u;
interface
type
  tbox = record
    v : longint;
  end;
  tregs = set of 0..3;
operator + (const a,b : tbox) : tregs;
procedure p;
implementation
operator + (const a,b : tbox) : tregs;
begin
  result := [0,2];
end;
procedure p;
var a,b : tbox; j : integer;
begin
  for j in a + b do
    j := j + 1;
end;
end.
