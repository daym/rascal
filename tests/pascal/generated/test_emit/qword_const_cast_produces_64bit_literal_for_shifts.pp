unit u;
interface
function shl_test(l : longint; d : longint) : qword;
implementation
function shl_test(l : longint; d : longint) : qword;
begin
  shl_test := qword(1) shl (32 + l) div d;
end;
end.
