unit ops;
interface
type
  tbox = record
    v : longint;
  end;
operator * (const a,b : tbox) : tbox;
operator := (const n : longint) : tbox;
function pick(n : longint) : longint; overload;
function pick(b : tbox) : tbox; overload;
procedure test;
implementation
operator * (const a,b : tbox) : tbox;
begin
  result.v := a.v * b.v;
end;
operator := (const n : longint) : tbox;
begin
  result.v := n;
end;
function pick(n : longint) : longint;
begin
  result := n;
end;
function pick(b : tbox) : tbox;
begin
  result := b;
end;
procedure test;
var a,b : tbox;
begin
  a := pick(b);
  a := pick(b) * 2;
end;
end.
