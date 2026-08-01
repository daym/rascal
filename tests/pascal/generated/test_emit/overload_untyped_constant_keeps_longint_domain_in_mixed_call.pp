unit u;
interface
function pair(a,b : qword) : shortstring; overload;
function pair(a,b : int64) : shortstring; overload;
function pair(a,b : longint) : shortstring; overload;
procedure run(c : cardinal);
implementation
function pair(a,b : qword) : shortstring; begin pair := ''; end;
function pair(a,b : int64) : shortstring; begin pair := ''; end;
function pair(a,b : longint) : shortstring; begin pair := ''; end;
procedure run(c : cardinal);
var s : shortstring;
begin
  s := pair(c, 1);
end;
end.
