unit u;
interface
type tbox = record v : longint; end;
operator := (const b : tbox) : qword;
function take(i : int64) : shortstring; overload;
function take(i : qword) : shortstring; overload;
procedure run(b : tbox);
implementation
operator := (const b : tbox) : qword;
begin
  result := b.v;
end;
function take(i : int64) : shortstring; begin take := ''; end;
function take(i : qword) : shortstring; begin take := ''; end;
procedure run(b : tbox);
begin
  take(b);
end;
end.
