unit ops;
interface
type
  tbox = record
    v : longint;
  end;
operator + (const a,b : tbox) : tbox;
operator - (const a,b : tbox) : tbox;
operator = (const a,b : tbox) : boolean;
operator div (const a,b : tbox) : tbox;
operator / (const a,b : tbox) : real;
operator := (const n : longint) : tbox;
operator := (const n : qword) : tbox;
operator := (const b : tbox) : longint;
operator := (const b : tbox) : int64;
type tarr = array[0..(high(qword) div 4)-1] of longint;
type tbase = class
end;
type tnode = class(tbase)
  value : tbox;
end;
procedure test;
implementation
operator + (const a,b : tbox) : tbox;
begin
  result.v := a.v + b.v;
end;
operator - (const a,b : tbox) : tbox;
begin
  result.v := a.v - b.v;
end;
operator = (const a,b : tbox) : boolean;
begin
  result := a.v = b.v;
end;
operator div (const a,b : tbox) : tbox;
begin
  result.v := a.v div b.v;
end;
operator / (const a,b : tbox) : real;
begin
  result := a.v / b.v;
end;
operator := (const n : longint) : tbox;
begin
  result.v := n;
end;
operator := (const n : qword) : tbox;
begin
  result.v := longint(n);
end;
operator := (const b : tbox) : longint;
begin
  result := b.v;
end;
operator := (const b : tbox) : int64;
begin
  result := b.v;
end;
procedure test;
const limit = high(longint);
var a,b,c : tbox; r : real; i : longint; base : tbase;
begin
  a := 1;
  b := a + a;
  c := a div b;
  c := 1 div b;
  c := a div sizeof(tbox);
  c := high(qword) div b;
  c := limit div b;
  c := tnode(base).value div 0;
  r := a / b;
  r := 1 / b;
  if a <> 0 then i := 1;
  if tnode(base).value <> 0 then i := 2;
  i := longint(b);
end;
end.
