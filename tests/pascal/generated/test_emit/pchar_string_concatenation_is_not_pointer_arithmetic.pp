unit u;
interface
function join(const prefix : ansistring; suffix : pchar) : ansistring;
implementation
function join(const prefix : ansistring; suffix : pchar) : ansistring;
begin
  join := prefix + suffix + #0;
end;
end.
