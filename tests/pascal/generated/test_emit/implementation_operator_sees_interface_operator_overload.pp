unit ops;
interface
type
  tbox = record
    v : longint;
  end;
operator + (const a,b : tbox) : tbox;
procedure test;
implementation
operator + (const a,b : tbox) : tbox;
begin
  result.v := a.v + b.v;
end;
operator + (const a : tbox; const n : longint) : tbox;
begin
  result.v := a.v + n;
end;
procedure test;
var a,b : tbox;
begin
  b := a + a;
  b := a + 1;
end;
end.
