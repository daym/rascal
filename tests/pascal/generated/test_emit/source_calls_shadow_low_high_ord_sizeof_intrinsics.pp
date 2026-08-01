unit u;
interface
function low(x : longint) : longint;
function high(x : longint) : longint;
function ord(x : longint) : longint;
function sizeof(x : longint) : longint;
procedure run;
implementation
function low(x : longint) : longint;
begin
  low := x;
end;
function high(x : longint) : longint;
begin
  high := x;
end;
function ord(x : longint) : longint;
begin
  ord := x;
end;
function sizeof(x : longint) : longint;
begin
  sizeof := x;
end;
procedure run;
var x : longint;
begin
  x := low(1) + high(2) + ord(3) + sizeof(4);
end;
end.
