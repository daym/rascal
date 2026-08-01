unit u;
interface
type
  tbytes = array[0..255] of byte;
  pstring = ^string;
function load(var bytes : tbytes; i : longint) : string;
implementation
function load(var bytes : tbytes; i : longint) : string;
begin
  load := pstring(@bytes[i])^;
end;
end.
