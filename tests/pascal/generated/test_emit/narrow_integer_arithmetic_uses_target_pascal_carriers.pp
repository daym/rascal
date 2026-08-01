unit u;
interface
function signedsum(a, b : smallint) : int64;
function unsignedsum(a, b : word) : qword;
function unsigneddifference(a, b : word) : int64;
implementation
function signedsum(a, b : smallint) : int64;
begin signedsum := a + b; end;
function unsignedsum(a, b : word) : qword;
begin unsignedsum := a + b; end;
function unsigneddifference(a, b : word) : int64;
begin unsigneddifference := a - b; end;
end.
