unit u;
interface
function tostr(i : qword) : string;
function tostr(i : int64) : string;
function tostr(i : longint) : string;
procedure run(v : cardinal);
implementation
function tostr(i : qword) : string;
begin
  tostr := '';
end;
function tostr(i : int64) : string;
begin
  tostr := '';
end;
function tostr(i : longint) : string;
begin
  tostr := '';
end;
procedure run(v : cardinal);
begin
  tostr(v);
end;
end.
