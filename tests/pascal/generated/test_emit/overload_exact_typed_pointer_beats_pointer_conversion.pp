unit u;
interface
type
  pint = ^longint;
function pick(p : pointer) : longint; overload;
function pick(p : pint) : longint; overload;
procedure run(raw : pointer; typed : pint; var a, b : longint);
implementation
function pick(p : pointer) : longint;
begin
  pick := 1;
end;
function pick(p : pint) : longint;
begin
  pick := 2;
end;
procedure run(raw : pointer; typed : pint; var a, b : longint);
begin
  a := pick(raw);
  b := pick(typed);
end;
end.
