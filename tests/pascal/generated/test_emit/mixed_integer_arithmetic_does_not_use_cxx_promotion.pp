unit u;
interface
function mixed(a : longword; b : shortint) : int64;
function wideunsigned(a : qword; b : shortint) : qword;
implementation
function mixed(a : longword; b : shortint) : int64;
begin mixed := a + b; end;
function wideunsigned(a : qword; b : shortint) : qword;
begin wideunsigned := a + b; end;
end.
